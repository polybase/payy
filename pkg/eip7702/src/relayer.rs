// lint-long-file-override allow-max-lines=750
use std::time::Duration;

use contextful::ResultContextExt;
use eth_util::secret_key_to_address;
use ethereum_types::{Address, H256, U256};
use secp256k1::{Message, Secp256k1, SecretKey};
use serde_json::json;
use tracing::warn;
use web3::{Web3, transports::Http};

use crate::auth::sign_authorization;
use crate::eip712::{encode_execute_meta_many_call, execute_meta_many_digest};
use crate::error::{Error, Result};
use crate::txn::{build_set_code_tx_json, sign_eip1559_envelope, sign_setcode_type4_envelope};
use crate::types::{Authorization, Eip7702Relayer, MetaCall};

const DEFAULT_META_GAS: U256 = U256([1_000_000, 0, 0, 0]);
// NOTE: DEFAULT_META_GAS may become configurable once production metrics are available.

pub struct HttpEip7702Relayer {
    pub(crate) rpc_url: String,
    pub(crate) http: reqwest::Client,
    pub(crate) relayer_sk: Option<SecretKey>,
}

impl HttpEip7702Relayer {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            http: reqwest::Client::new(),
            relayer_sk: None,
        }
    }

    pub fn new_with_signer(rpc_url: impl Into<String>, relayer_sk: SecretKey) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            http: reqwest::Client::new(),
            relayer_sk: Some(relayer_sk),
        }
    }

    #[allow(clippy::result_large_err)]
    fn web3(&self) -> Result<Web3<Http>> {
        let transport = Http::new(&self.rpc_url).context("create web3 http transport")?;
        Ok(Web3::new(transport))
    }

    async fn priority_fee_hint(&self) -> Result<Option<U256>> {
        let req = serde_json::json!({ "jsonrpc": "2.0", "method": "eth_maxPriorityFeePerGas", "params": [], "id": 1 });
        let res = self
            .http
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .context("post eth_maxPriorityFeePerGas")?;
        let v: serde_json::Value = res
            .json()
            .await
            .context("decode eth_maxPriorityFeePerGas response")?;
        let Some(raw) = v.get("result") else {
            return Ok(None);
        };
        let Some(hex_str) = raw.as_str() else {
            return Ok(None);
        };
        let trimmed = hex_str.trim_start_matches("0x");
        if trimmed.is_empty() {
            return Ok(Some(U256::zero()));
        }
        let parsed = U256::from_str_radix(trimmed, 16)
            .with_context(|| format!("parse eth_maxPriorityFeePerGas result {hex_str}"))?;
        Ok(Some(parsed))
    }

    async fn suggest_fees_fast(&self) -> Result<(U256, U256)> {
        let web3 = self.web3()?;
        let gp = web3
            .eth()
            .gas_price()
            .await
            .unwrap_or_else(|_| U256::from(1_500_000_000u64));

        let mut prio = match self.priority_fee_hint().await {
            Ok(Some(hint)) => hint,
            Ok(None) => gp / 4,
            Err(err) => {
                warn!(?err, "failed to fetch eth_maxPriorityFeePerGas hint");
                gp / 4
            }
        };

        if prio.is_zero() {
            prio = U256::from(1u64);
        }

        let max_fee = gp.saturating_add(prio);
        Ok((prio, max_fee))
    }

    async fn send_tx_payload(&self, payload: serde_json::Value) -> Result<H256> {
        let req = json!({ "jsonrpc": "2.0", "method": "eth_sendTransaction", "params": [payload], "id": 1 });
        let res = self
            .http
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .context("post eth_sendTransaction")?;

        let v: serde_json::Value = res
            .json()
            .await
            .context("decode eth_sendTransaction response")?;
        if let Some(s) = v.get("result").and_then(|x| x.as_str()) {
            let trimmed = s.strip_prefix("0x").unwrap_or(s);
            let bytes = hex::decode(trimmed)
                .with_context(|| format!("decode eth_sendTransaction result {s}"))?;
            if bytes.len() != 32 {
                Err(Error::TxHashLength {
                    hash: s.to_owned(),
                    length: bytes.len(),
                })
            } else {
                Ok(H256::from_slice(&bytes))
            }
        } else {
            Err(Error::RpcError {
                response: v.to_string(),
            })
        }
    }

    async fn estimate_gas_with_overrides(
        &self,
        txn: serde_json::Value,
        override_addr: Address,
        override_balance_wei: U256,
    ) -> Result<U256> {
        let overrides = serde_json::json!({ format!("0x{override_addr:x}"): { "balance": format!("0x{override_balance_wei:x}") } });
        let req = serde_json::json!({ "jsonrpc": "2.0", "method": "eth_estimateGas", "params": [txn, "latest", overrides], "id": 1 });
        let res = self
            .http
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .context("post eth_estimateGas with overrides")?;
        let v: serde_json::Value = res
            .json()
            .await
            .context("decode eth_estimateGas with overrides response")?;

        if let Some(s) = v.get("result").and_then(|x| x.as_str()) {
            let trimmed = s.trim_start_matches("0x");
            if trimmed.is_empty() {
                Ok(U256::zero())
            } else {
                Ok(U256::from_str_radix(trimmed, 16)
                    .with_context(|| format!("parse eth_estimateGas with overrides result {s}"))?)
            }
        } else {
            Err(Error::EstimateGasError {
                response: v.to_string(),
            })
        }
    }

    async fn estimate_gas(&self, payload: serde_json::Value) -> Result<U256> {
        let req = serde_json::json!({ "jsonrpc": "2.0", "method": "eth_estimateGas", "params": [payload], "id": 1 });
        let res = self
            .http
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .context("post eth_estimateGas")?;
        let v: serde_json::Value = res
            .json()
            .await
            .context("decode eth_estimateGas response")?;

        if let Some(s) = v.get("result").and_then(|x| x.as_str()) {
            let trimmed = s.trim_start_matches("0x");
            if trimmed.is_empty() {
                Ok(U256::zero())
            } else {
                Ok(U256::from_str_radix(trimmed, 16)
                    .with_context(|| format!("parse eth_estimateGas result {s}"))?)
            }
        } else {
            Err(Error::EstimateGasError {
                response: v.to_string(),
            })
        }
    }

    async fn sign_eip1559_raw(
        &self,
        relayer_sk: &SecretKey,
        to: Address,
        data: Vec<u8>,
        gas: U256,
        value: U256,
    ) -> Result<Vec<u8>> {
        let web3 = self.web3()?;
        let chain_id = web3
            .eth()
            .chain_id()
            .await
            .context("fetch chain_id for eip1559 envelope")?;
        let relayer_addr = secret_key_to_address(relayer_sk);

        let nonce = web3
            .eth()
            .transaction_count(relayer_addr, Some(web3::types::BlockNumber::Pending))
            .await
            .context("fetch relayer pending nonce")?;
        let (max_prio, max_fee) = self.suggest_fees_fast().await?;

        Ok(sign_eip1559_envelope(
            chain_id, nonce, max_prio, max_fee, gas, to, value, data, relayer_sk,
        ))
    }

    async fn sign_setcode_type4_raw(
        &self,
        relayer_sk: &SecretKey,
        user: Address,
        gas: U256,
        max_prio: U256,
        max_fee: U256,
        auth: &Authorization,
    ) -> Result<Vec<u8>> {
        let web3 = self.web3()?;
        let chain_id = web3
            .eth()
            .chain_id()
            .await
            .context("fetch chain_id for setcode envelope")?;
        let relayer_addr = secret_key_to_address(relayer_sk);
        let nonce = web3
            .eth()
            .transaction_count(relayer_addr, Some(web3::types::BlockNumber::Pending))
            .await
            .context("fetch relayer pending nonce")?;

        Ok(sign_setcode_type4_envelope(
            chain_id, nonce, max_prio, max_fee, gas, user, auth, relayer_sk,
        ))
    }

    /// Quick pre-check to see if `user` code already reflects delegation to `delegate`.
    pub async fn is_delegated_to(&self, user: Address, delegate: Address) -> Result<bool> {
        Ok(matches!(
            self.get_delegate_of(user).await?,
            Some(current) if current == delegate
        ))
    }

    pub async fn get_delegate_of(&self, user: Address) -> Result<Option<Address>> {
        let web3 = self.web3()?;
        let code = web3
            .eth()
            .code(user, None)
            .await
            .context("fetch account code")?;
        let b = code.0;
        if b.len() >= 23 && b[0..3] == [0xef, 0x01, 0x00] {
            let addr_bytes = &b[3..23];
            let addr = Address::from_slice(addr_bytes);
            Ok(Some(addr))
        } else {
            Ok(None)
        }
    }

    /// Send a SetCode (type 0x04) transaction using a user-provided Authorization tuple.
    pub async fn send_setcode_with_authorization(
        &self,
        user: Address,
        auth: &Authorization,
    ) -> Result<H256> {
        let web3 = self.web3()?;
        // Ensure the network chain_id matches the authorization tuple's chain_id
        let network_chain_id = web3
            .eth()
            .chain_id()
            .await
            .context("fetch network chain id")?;
        if network_chain_id != auth.chain_id {
            return Err(Error::ChainIdMismatch {
                authorization: auth.chain_id,
                network: network_chain_id,
            });
        }

        // Try to estimate SetCode gas with state override to avoid funds checks on the relayer
        let relayer_addr = self
            .relayer_sk
            .as_ref()
            .map(secret_key_to_address)
            .ok_or(Error::RelayerNotConfigured)?;

        // Minimal fee envelope for estimation to satisfy basefee constraints
        let gp = web3.eth().gas_price().await.context("fetch gas_price")?;
        let est_prio = U256::from(1_000_000_000u64).min(gp); // 1 gwei or lower
        let est_max_fee = gp.saturating_add(est_prio);

        let relayer_nonce = web3
            .eth()
            .transaction_count(relayer_addr, Some(web3::types::BlockNumber::Pending))
            .await
            .context("fetch relayer pending nonce")?;

        let estimate_tx = build_set_code_tx_json(
            relayer_addr,
            user,
            network_chain_id,
            relayer_nonce,
            U256::zero(),
            est_max_fee,
            est_prio,
            auth,
        );
        // Override relayer balance to a large value (e.g. 100 ETH) for simulation only
        let override_balance = U256::from_dec_str("100000000000000000000").unwrap(); // 100 ETH
        let gas = match self
            .estimate_gas_with_overrides(estimate_tx, relayer_addr, override_balance)
            .await
        {
            Ok(g) => g + g / 5u32, // +20% buffer
            Err(_) => U256::from(200_000u64),
        };

        // Compute fees for signing/broadcast (~1.5x policy)
        let (max_prio, max_fee) = self.suggest_fees_fast().await?;

        let sk = self
            .relayer_sk
            .as_ref()
            .ok_or(Error::RelayerNotConfigured)?;

        let raw = self
            .sign_setcode_type4_raw(sk, user, gas, max_prio, max_fee, auth)
            .await?;
        let req = json!({ "jsonrpc": "2.0", "method": "eth_sendRawTransaction", "params": [format!("0x{}", hex::encode(raw))], "id": 1 });
        let res = self
            .http
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .context("post eth_sendRawTransaction for setcode")?;
        let v: serde_json::Value = res
            .json()
            .await
            .context("decode eth_sendRawTransaction setcode response")?;

        if let Some(s) = v.get("result").and_then(|x| x.as_str()) {
            let trimmed = s.strip_prefix("0x").unwrap_or(s);
            let bytes = hex::decode(trimmed)
                .with_context(|| format!("decode eth_sendRawTransaction setcode result {s}"))?;
            if bytes.len() != 32 {
                Err(Error::TxHashLength {
                    hash: s.to_owned(),
                    length: bytes.len(),
                })
            } else {
                Ok(H256::from_slice(&bytes))
            }
        } else {
            Err(Error::RpcRawError {
                response: v.to_string(),
            })
        }
    }

    /// Send batched executeMeta with an already signed user signature (no user_sk needed on server).
    pub async fn send_execute_meta_many_with_signature(
        &self,
        user: Address,
        calls: Vec<MetaCall>,
        nonce: U256,
        valid_after: U256,
        valid_until: U256,
        signature: Vec<u8>,
    ) -> Result<H256> {
        let sk = self
            .relayer_sk
            .as_ref()
            .ok_or(Error::RelayerNotConfigured)?;
        // Preflight estimate for meta call execution (from relayer to user)
        let relayer_addr = secret_key_to_address(sk);
        let call_data =
            encode_execute_meta_many_call(calls, nonce, valid_after, valid_until, signature);
        let estimate_payload = build_execute_meta_estimate_payload(relayer_addr, user, &call_data);

        let mut gas = self.estimate_gas(estimate_payload).await?;
        gas = gas + gas / 2u32;
        let raw = self
            .sign_eip1559_raw(sk, user, call_data, gas, U256::zero())
            .await?;
        let req = json!({ "jsonrpc": "2.0", "method": "eth_sendRawTransaction", "params": [format!("0x{}", hex::encode(raw))], "id": 1 });
        let res = self
            .http
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .context("post eth_sendRawTransaction for executeMetaMany")?;
        let v: serde_json::Value = res
            .json()
            .await
            .context("decode eth_sendRawTransaction executeMetaMany response")?;
        if let Some(s) = v.get("result").and_then(|x| x.as_str()) {
            let trimmed = s.strip_prefix("0x").unwrap_or(s);
            let bytes = hex::decode(trimmed).with_context(|| {
                format!("decode eth_sendRawTransaction executeMetaMany result {s}")
            })?;
            if bytes.len() != 32 {
                Err(Error::TxHashLength {
                    hash: s.to_owned(),
                    length: bytes.len(),
                })
            } else {
                Ok(H256::from_slice(&bytes))
            }
        } else {
            Err(Error::RpcRawError {
                response: v.to_string(),
            })
        }
    }
}

fn fmt_u256_hex(v: &U256) -> String {
    if v.is_zero() {
        "0x0".to_string()
    } else {
        format!("0x{v:x}")
    }
}

fn build_execute_meta_estimate_payload(
    relayer_addr: Address,
    user: Address,
    call_data: &[u8],
) -> serde_json::Value {
    serde_json::json!({
        "from": format!("0x{relayer_addr:x}"),
        "to": format!("0x{user:x}"),
        "value": "0x0",
        "data": format!("0x{}", hex::encode(call_data)),
    })
}

#[async_trait::async_trait]
impl Eip7702Relayer for HttpEip7702Relayer {
    async fn ensure_upgraded(
        &self,
        user_sk: &SecretKey,
        user: Address,
        delegate: Address,
        relayer: Address,
    ) -> Result<H256> {
        let web3 = self.web3()?;
        // Pre-check: if user's code already reflects delegation to `delegate`, skip sending
        let code = web3
            .eth()
            .code(user, None)
            .await
            .context("fetch user account code")?;
        let b = code.0;
        if b.len() >= 23 && b[0..3] == [0xef, 0x01, 0x00] && &b[3..23] == delegate.as_bytes() {
            return Ok(H256::zero());
        }
        let chain_id = web3
            .eth()
            .chain_id()
            .await
            .context("fetch chain_id for ensure_upgraded")?;
        let user_nonce = web3
            .eth()
            .transaction_count(user, Some(web3::types::BlockNumber::Latest))
            .await
            .context("fetch user latest nonce")?;
        let relayer_nonce = web3
            .eth()
            .transaction_count(relayer, Some(web3::types::BlockNumber::Pending))
            .await
            .context("fetch relayer pending nonce")?;
        let auth = sign_authorization(user_sk, chain_id, delegate, user_nonce);
        let gas = U256::from(200_000u64);
        let (max_prio, max_fee) = self.suggest_fees_fast().await?;
        if let Some(sk) = &self.relayer_sk {
            let raw = self
                .sign_setcode_type4_raw(sk, user, gas, max_prio, max_fee, &auth)
                .await?;
            let req = json!({ "jsonrpc": "2.0", "method": "eth_sendRawTransaction", "params": [format!("0x{}", hex::encode(raw))], "id": 1 });
            let res = self
                .http
                .post(&self.rpc_url)
                .json(&req)
                .send()
                .await
                .context("post eth_sendRawTransaction for ensure_upgraded")?;
            let v: serde_json::Value = res
                .json()
                .await
                .context("decode eth_sendRawTransaction ensure_upgraded response")?;
            if let Some(s) = v.get("result").and_then(|x| x.as_str()) {
                let trimmed = s.strip_prefix("0x").unwrap_or(s);
                let bytes = hex::decode(trimmed).with_context(|| {
                    format!("decode eth_sendRawTransaction ensure_upgraded result {s}")
                })?;
                if bytes.len() != 32 {
                    Err(Error::TxHashLength {
                        hash: s.to_owned(),
                        length: bytes.len(),
                    })
                } else {
                    Ok(H256::from_slice(&bytes))
                }
            } else {
                Err(Error::RpcRawError {
                    response: v.to_string(),
                })
            }
        } else {
            let payload = build_set_code_tx_json(
                relayer,
                user,
                chain_id,
                relayer_nonce,
                gas,
                max_fee,
                max_prio,
                &auth,
            );
            self.send_tx_payload(payload).await
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_meta_tx(
        &self,
        user_sk: &SecretKey,
        user: Address,
        calls: Vec<MetaCall>,
        relayer: Address,
        nonce: U256,
        valid_after: U256,
        valid_until: U256,
    ) -> Result<H256> {
        let web3 = self.web3()?;
        let chain_id = web3
            .eth()
            .chain_id()
            .await
            .context("fetch chain_id for send_meta_tx")?;
        let meta_nonce = nonce;
        let digest =
            execute_meta_many_digest(chain_id, user, &calls, meta_nonce, valid_after, valid_until);
        let sig = Secp256k1::new().sign_ecdsa_recoverable(
            &Message::from_digest_slice(digest.as_bytes()).unwrap(),
            user_sk,
        );
        let (rid, bytes) = sig.serialize_compact();
        let mut signature = Vec::with_capacity(65);
        signature.extend_from_slice(&bytes);
        // Ethereum expects v in {27,28}; convert recover id {0,1} accordingly
        signature.push(27 + ((rid.to_i32() as u8) & 1));
        let call_data =
            encode_execute_meta_many_call(calls, meta_nonce, valid_after, valid_until, signature);
        if let Some(sk) = &self.relayer_sk {
            let gas = DEFAULT_META_GAS;
            let raw = self
                .sign_eip1559_raw(sk, user, call_data, gas, U256::zero())
                .await?;
            let req = json!({ "jsonrpc": "2.0", "method": "eth_sendRawTransaction", "params": [format!("0x{}", hex::encode(raw))], "id": 1 });
            let res = self
                .http
                .post(&self.rpc_url)
                .json(&req)
                .send()
                .await
                .context("post eth_sendRawTransaction for send_meta_tx")?;
            let v: serde_json::Value = res
                .json()
                .await
                .context("decode eth_sendRawTransaction send_meta_tx response")?;
            if let Some(s) = v.get("result").and_then(|x| x.as_str()) {
                let trimmed = s.strip_prefix("0x").unwrap_or(s);
                let bytes = hex::decode(trimmed).with_context(|| {
                    format!("decode eth_sendRawTransaction send_meta_tx result {s}")
                })?;
                if bytes.len() != 32 {
                    Err(Error::TxHashLength {
                        hash: s.to_owned(),
                        length: bytes.len(),
                    })
                } else {
                    Ok(H256::from_slice(&bytes))
                }
            } else {
                Err(Error::RpcRawError {
                    response: v.to_string(),
                })
            }
        } else {
            let gas = DEFAULT_META_GAS;
            let payload = json!({
                "from": format!("0x{relayer:x}"),
                "to": format!("0x{user:x}"),
                "gas": fmt_u256_hex(&gas),
                "value": fmt_u256_hex(&U256::zero()),
                "data": format!("0x{}", hex::encode(call_data))
            });
            self.send_tx_payload(payload).await
        }
    }

    async fn wait_for_upgrade(
        &self,
        user: Address,
        delegate: Address,
        timeout: Duration,
    ) -> Result<()> {
        let web3 = self.web3()?;
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            let code = web3
                .eth()
                .code(user, None)
                .await
                .context("fetch user account code")?;
            let b = code.0;
            if b.len() >= 23 && b[0..3] == [0xef, 0x01, 0x00] && &b[3..23] == delegate.as_bytes() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(Error::DelegationTimeout)
    }
}

#[cfg(test)]
#[path = "relayer_test.rs"]
mod relayer_test;
