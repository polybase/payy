use std::sync::Arc;

use async_trait::async_trait;
use contextful::ResultContextExt;
use payy_evm_client_interface::{
    Address, B256, PayyBlockTag, PayyEvmSubmitter, PayyEvmTransactionRequest,
    PayyRawTransactionSubmitter, PayyTransactionCountArgs, Result, TxHash,
};
use rlp::RlpStream;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

use crate::util::keccak32;

/// Local EVM signer backed by a raw-transaction broadcaster.
pub struct LocalRawEvmSubmitter {
    evm_private_key: B256,
    raw_submitter: Arc<dyn PayyRawTransactionSubmitter>,
}

impl LocalRawEvmSubmitter {
    /// Create a local raw-transaction submitter.
    #[must_use]
    pub fn new(evm_private_key: B256, raw_submitter: Arc<dyn PayyRawTransactionSubmitter>) -> Self {
        Self {
            evm_private_key,
            raw_submitter,
        }
    }

    fn signer(&self) -> Result<SecretKey> {
        SecretKey::from_slice(&self.evm_private_key)
            .context("construct secp256k1 private key signer")
            .map_err(Into::into)
    }

    fn address(&self) -> Result<Address> {
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &self.signer()?);
        Ok(evm_address_from_public_key(&public_key))
    }
}

#[async_trait]
impl PayyEvmSubmitter for LocalRawEvmSubmitter {
    async fn get_chain_id(&self) -> Result<u64> {
        self.raw_submitter.get_chain_id().await
    }

    async fn get_address(&self) -> Result<Option<Address>> {
        Ok(Some(self.address()?))
    }

    async fn send_transaction(&self, request: PayyEvmTransactionRequest) -> Result<TxHash> {
        let from = match request.from {
            Some(from) => from,
            None => self.address()?,
        };
        if from != self.address()? {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: payy_evm_client_interface::ValidationErrorKind::EvmAccountMismatch,
            });
        }
        let request = PayyEvmTransactionRequest {
            from: Some(from),
            ..request
        };
        let chain_id = self.raw_submitter.get_chain_id().await?;
        let nonce = self
            .raw_submitter
            .get_transaction_count(PayyTransactionCountArgs {
                address: from,
                block_tag: PayyBlockTag::Pending,
            })
            .await?;
        let gas_limit = match request.gas_limit {
            Some(gas_limit) => gas_limit,
            None => self.raw_submitter.estimate_gas(request.clone()).await?,
        };
        let fee_data = self.raw_submitter.get_fee_data().await?;
        let tx = Eip1559Transaction {
            chain_id,
            nonce,
            gas_limit,
            max_fee_per_gas: fee_data.max_fee_per_gas,
            max_priority_fee_per_gas: fee_data.max_priority_fee_per_gas,
            to: request.to,
            value: request.value,
            input: request.data,
        };
        let signer = self.signer()?;
        let raw = tx.sign(&signer)?;
        self.raw_submitter.send_raw_transaction(raw).await
    }
}

struct Eip1559Transaction {
    chain_id: u64,
    nonce: u64,
    gas_limit: u64,
    max_fee_per_gas: u128,
    max_priority_fee_per_gas: u128,
    to: Address,
    value: u128,
    input: Vec<u8>,
}

impl Eip1559Transaction {
    fn sign(&self, signer: &SecretKey) -> Result<Vec<u8>> {
        let sighash = keccak32(&[&self.signing_payload()]);
        let message = Message::from_digest_slice(&sighash).context("build eip1559 sighash")?;
        let signature = Secp256k1::new().sign_ecdsa_recoverable(&message, signer);
        let (recovery_id, signature_bytes) = signature.serialize_compact();
        let y_parity =
            u8::try_from(recovery_id.to_i32()).context("convert eip1559 recovery id")? & 1;

        let mut body = self.rlp_unsigned_body(12);
        body.append(&y_parity);
        body.append(&trim_leading_zeroes(&signature_bytes[..32]));
        body.append(&trim_leading_zeroes(&signature_bytes[32..]));

        let out = body.out();
        let mut raw = Vec::with_capacity(1 + out.len());
        raw.push(0x02);
        raw.extend_from_slice(&out);
        Ok(raw)
    }

    fn signing_payload(&self) -> Vec<u8> {
        let body = self.rlp_unsigned_body(9);
        let out = body.out();
        let mut payload = Vec::with_capacity(1 + out.len());
        payload.push(0x02);
        payload.extend_from_slice(&out);
        payload
    }

    fn rlp_unsigned_body(&self, list_len: usize) -> RlpStream {
        let mut body = RlpStream::new_list(list_len);
        body.append(&int_bytes(self.chain_id));
        body.append(&int_bytes(self.nonce));
        body.append(&int_bytes(self.max_priority_fee_per_gas));
        body.append(&int_bytes(self.max_fee_per_gas));
        body.append(&int_bytes(self.gas_limit));
        body.append(&self.to.as_ref());
        body.append(&int_bytes(self.value));
        body.append(&self.input);
        let access_list = RlpStream::new_list(0);
        body.append_raw(&access_list.out(), 1);
        body
    }
}

fn evm_address_from_public_key(public_key: &PublicKey) -> Address {
    let public_key = public_key.serialize_uncompressed();
    let hash = keccak32(&[&public_key[1..]]);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);
    address
}

fn int_bytes(value: impl Into<u128>) -> Vec<u8> {
    trim_leading_zeroes(&value.into().to_be_bytes())
}

fn trim_leading_zeroes(bytes: &[u8]) -> Vec<u8> {
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    bytes[first..].to_vec()
}
