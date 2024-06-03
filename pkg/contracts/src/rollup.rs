use std::sync::Arc;
use std::time::Duration;

use crate::constants::{AGG_INSTANCES, UTXO_INPUTS, UTXO_N};
use crate::error::Result;
use crate::util::convert_element_to_h256;
use crate::Client;
use ethereum_types::{H160, H256, U256, U64};
use parking_lot::RwLock;
use secp256k1::{Message, SECP256K1};
use sha3::{Digest, Keccak256};
use tracing::warn;
use web3::contract::tokens::{Tokenizable, TokenizableItem, Tokenize};
use web3::ethabi::Token;
use web3::futures::{Stream, StreamExt};
use web3::signing::SecretKeyRef;
use web3::transports::Http;
use web3::types::FilterBuilder;
use web3::{
    contract::Contract,
    signing::{Key, SecretKey},
    types::Address,
};
use zk_primitives::Element;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatorSet {
    pub validators: Vec<Address>,
    pub valid_from: U256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Burn {
    pub to: H160,
    pub amount: U256,
}

impl From<(H160, U256)> for Burn {
    fn from(item: (H160, U256)) -> Self {
        Self {
            to: item.0,
            amount: item.1,
        }
    }
}

impl Tokenizable for Burn {
    fn from_token(token: Token) -> Result<Self, web3::contract::Error>
    where
        Self: Sized,
    {
        match token {
            Token::Tuple(tokens) => {
                if tokens.len() != 2 {
                    return Err(web3::contract::Error::InvalidOutputType(
                        "expected tuple of length 2".to_string(),
                    ));
                }

                let mut tokens = tokens.into_iter();
                let (to, amount) = (tokens.next().unwrap(), tokens.next().unwrap());

                let to = H160::from_token(to)?;
                let amount = U256::from_token(amount)?;

                Ok(Self { to, amount })
            }
            _ => Err(web3::contract::Error::InvalidOutputType(
                "expected tuple".to_string(),
            )),
        }
    }

    fn into_token(self) -> Token {
        Token::Tuple(vec![Token::Address(self.to), Token::Uint(self.amount)])
    }
}

impl Tokenizable for ValidatorSet {
    fn from_token(token: Token) -> Result<Self, web3::contract::Error>
    where
        Self: Sized,
    {
        match token {
            Token::Tuple(tokens) => {
                if tokens.len() != 2 {
                    return Err(web3::contract::Error::InvalidOutputType(
                        "expected tuple of length 2".to_string(),
                    ));
                }

                let mut tokens = tokens.into_iter();
                let (validators, valid_from) = (tokens.next().unwrap(), tokens.next().unwrap());

                let validators = Vec::<Address>::from_token(validators)?;
                let valid_from = U256::from_token(valid_from)?;

                Ok(Self {
                    validators,
                    valid_from,
                })
            }
            _ => Err(web3::contract::Error::InvalidOutputType(
                "expected tuple".to_string(),
            )),
        }
    }

    fn into_token(self) -> Token {
        Token::Tuple(vec![
            Token::Array(self.validators.into_tokens()),
            Token::Uint(self.valid_from),
        ])
    }
}

impl TokenizableItem for ValidatorSet {}

#[derive(Clone, Debug)]
pub struct RollupContract {
    pub client: Client,
    pub contract: Contract<Http>,
    pub signer: SecretKey,
    signer_address: Address,
    pub domain_separator: H256,
    pub validator_sets: Arc<RwLock<Vec<ValidatorSet>>>,
    address: Address,
    /// The ethereum block height used for all contract calls.
    /// If None, the latest block is used.
    block_height: Option<U64>,
}

impl RollupContract {
    pub fn new(
        client: Client,
        contract: Contract<Http>,
        signer: SecretKey,
        domain_separator: H256,
        address: Address,
    ) -> Self {
        let signer_address = Key::address(&SecretKeyRef::new(&signer));

        Self {
            client,
            contract,
            signer,
            signer_address,
            domain_separator,
            validator_sets: Arc::new(RwLock::new(Vec::new())),
            address,
            block_height: None,
        }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub async fn load(
        client: Client,
        rollup_contract_addr: &str,
        signer: SecretKey,
    ) -> Result<Self> {
        let contract_json =
            include_str!("../../../eth/artifacts/contracts/rollup/RollupV1.sol/RollupV1.json");
        let contract = client.load_contract_from_str(rollup_contract_addr, contract_json)?;

        let domain_separator = client
            .query::<H256, _, _, _>(
                &contract,
                "DOMAIN_SEPARATOR",
                (),
                None,
                Default::default(),
                None,
            )
            .await?;

        let self_ = Self::new(
            client,
            contract,
            signer,
            domain_separator,
            rollup_contract_addr.parse()?,
        );
        // The node expects validator_sets to be available immediately, so we set it here
        self_.load_all_validators().await?;

        Ok(self_)
    }

    pub fn at_height(self, height: Option<u64>) -> Self {
        Self {
            block_height: height.map(|x| x.into()),
            ..self
        }
    }

    async fn load_all_validators(&self) -> Result<()> {
        let all_validators = self.get_validator_sets(0).await?;
        *self.validator_sets.write() = all_validators;
        Ok(())
    }

    pub async fn worker(&self, interval: Duration) -> Result<()> {
        let mut events = self.listen_for_validator_set_added(interval).await?.boxed();

        let this = self.clone();
        let mut consecutive_transport_error_count = 0;
        const MAX_CONSECUTIVE_TRANSPORT_ERRORS: u64 = 5;
        tokio::spawn(async move {
            while let Some(event) = events.next().await {
                let event = match event {
                    Ok(event) => {
                        consecutive_transport_error_count = 0;

                        event
                    },
                    Err(err @ web3::Error::Transport(_)) =>
                    {
                        // TODO: refactor this retry logic
                        consecutive_transport_error_count += 1;

                        if consecutive_transport_error_count > MAX_CONSECUTIVE_TRANSPORT_ERRORS {
                            return Err(err.into());
                        }

                        warn!(
                            ?err,
                            consecutive_transport_error_count,
                            "Received a transport error while listening for 'validator set added' events. Retrying."
                        );

                        events = loop {
                            tokio::time::sleep(interval).await;

                            match this.listen_for_validator_set_added(interval).await {
                                Ok(events) => break events.boxed(),
                                Err(err @ web3::Error::Transport(_)) => {
                                    consecutive_transport_error_count += 1;

                                    if consecutive_transport_error_count > MAX_CONSECUTIVE_TRANSPORT_ERRORS {
                                        return Err(err.into());
                                    }

                                    warn!(
                                        ?err,
                                        consecutive_transport_error_count,
                                        "Received a transport error while trying to create a new event listener. Retrying."
                                    );
                                    continue;
                                },
                                Err(err) => return Err(err.into()),
                            }
                        };

                        this.load_all_validators().await?;
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                };

                let index = U256::from_big_endian(&event.data.0[0..32]);
                let _valid_from = U256::from_big_endian(&event.data.0[32..64]);

                let current_last_index = this.validator_sets.read().len() - 1;
                if index.as_usize() > current_last_index {
                    // A new validator set was added to the contract
                    let new_validators = this
                        .get_validator_sets(current_last_index as u64 + 1)
                        .await?;
                    this.validator_sets.write().extend(new_validators);
                }
            }

            Ok(())
        })
        .await?
    }

    pub async fn call(&self, func: &str, params: impl Tokenize + Clone) -> Result<H256> {
        self.client
            .call(
                &self.contract,
                func,
                params,
                &self.signer,
                self.signer_address,
            )
            .await
    }

    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(err, ret, skip(self, proof))]
    pub async fn verify_block(
        &self,
        proof: &[u8],
        agg_instances: [Element; AGG_INSTANCES],
        old_root: &Element,
        new_root: &Element,
        // 6 utxo * 3 hashes per utxo
        utxo_inputs: &[Element],
        other_hash: [u8; 32],
        height: u64,
        signatures: &[&[u8]],
    ) -> Result<H256> {
        // Ensure we have the correct number of UTXO inputs
        assert_eq!(utxo_inputs.len(), UTXO_N * UTXO_INPUTS);

        let signatures = signatures
            .iter()
            .map(|sig| {
                let r = sig[0..32].to_vec();
                let s = sig[32..64].to_vec();
                let v = sig[64];
                let v = if v < 27 { v + 27 } else { v };

                Token::Tuple(vec![
                    Token::FixedBytes(r),
                    Token::FixedBytes(s),
                    Token::Uint(v.into()),
                ])
            })
            .collect::<Vec<Token>>();

        let utxo_hashes = utxo_inputs
            .iter()
            .map(convert_element_to_h256)
            .map(|x| Token::FixedBytes(x.as_bytes().to_vec()))
            .collect::<Vec<Token>>();

        let call_tx = self
            .call(
                "verifyBlock",
                (
                    web3::types::Bytes::from(proof),
                    agg_instances.map(|x| convert_element_to_h256(&x)),
                    convert_element_to_h256(old_root),
                    convert_element_to_h256(new_root),
                    Token::FixedArray(utxo_hashes),
                    H256::from_slice(&other_hash),
                    U256::from(height),
                    Token::Array(signatures),
                ),
            )
            .await?;

        Ok(call_tx)
    }

    #[tracing::instrument(err, ret, skip(self, proof))]
    pub async fn mint(
        &self,
        proof: &[u8],
        commitment: &Element,
        value: &Element,
        source: &Element,
    ) -> Result<H256> {
        let call_tx = self
            .call(
                "mint",
                (
                    web3::types::Bytes::from(proof),
                    convert_element_to_h256(commitment),
                    convert_element_to_h256(value),
                    convert_element_to_h256(source),
                ),
            )
            .await?;

        Ok(call_tx)
    }

    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(err, ret, skip(self, proof))]
    pub async fn mint_with_authorization(
        &self,
        proof: &[u8],
        commitment: &Element,
        value: &Element,
        source: &Element,
        from: &Address,
        // unix timestamp
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
        signature_for_receive: &[u8],
        signature_for_mint: &[u8],
    ) -> Result<H256> {
        let r = &signature_for_receive[0..32];
        let s = &signature_for_receive[32..64];
        let v = signature_for_receive[64];
        let v = if v < 27 { v + 27 } else { v };

        let r2 = &signature_for_mint[0..32];
        let s2 = &signature_for_mint[32..64];
        let v2 = signature_for_mint[64];
        let v2 = if v2 < 27 { v2 + 27 } else { v2 };

        let call_tx = self
            .call(
                "mintWithAuthorization",
                (
                    web3::types::Bytes::from(proof),
                    convert_element_to_h256(commitment),
                    convert_element_to_h256(value),
                    convert_element_to_h256(source),
                    web3::types::H160::from_slice(from.as_bytes()),
                    valid_after,
                    valid_before,
                    nonce,
                    web3::types::U256::from(v),
                    web3::types::H256::from_slice(r),
                    web3::types::H256::from_slice(s),
                    web3::types::U256::from(v2),
                    web3::types::H256::from_slice(r2),
                    web3::types::H256::from_slice(s2),
                ),
            )
            .await?;

        Ok(call_tx)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn signature_for_mint(
        &self,
        commitment: Element,
        value: U256,
        source: Element,
        from: Address,
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
        secret_key: secp256k1::SecretKey,
    ) -> [u8; 65] {
        // Sig for our mint function
        let mint_sig_digest = self.signature_msg_digest_for_mint(
            commitment,
            value,
            source,
            from,
            valid_after,
            valid_before,
            nonce,
        );

        let signature =
            SECP256K1.sign_ecdsa_recoverable(&Message::from_digest(mint_sig_digest), &secret_key);
        let mut mint_sig_bytes = [0u8; 65];
        let (recovery_id, signature) = signature.serialize_compact();
        mint_sig_bytes[0..64].copy_from_slice(&signature[0..64]);
        mint_sig_bytes[64] = recovery_id.to_i32() as u8;
        mint_sig_bytes
    }

    /// This signature authorizes another user to call mintWithAuthorization
    /// on behalf of the signer.
    #[allow(clippy::too_many_arguments)]
    pub fn signature_msg_digest_for_mint(
        &self,
        commitment: Element,
        value: U256,
        source: Element,
        from: Address,
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
    ) -> [u8; 32] {
        let mut data = Vec::new();
        let mint_with_authorization_typehash = Keccak256::digest(
            "MintWithAuthorization(bytes32 commitment,bytes32 value,bytes32 source,address from,uint256 validAfter,uint256 validBefore,bytes32 nonce)"
                .as_bytes(),
        );
        data.extend_from_slice(&mint_with_authorization_typehash);
        data.extend_from_slice(convert_element_to_h256(&commitment).as_bytes());
        let mut value_bytes = [0u8; 32];
        value.to_big_endian(&mut value_bytes);
        data.extend_from_slice(&value_bytes[..]);
        data.extend_from_slice(convert_element_to_h256(&source).as_bytes());
        data.extend_from_slice(H256::from(from).as_bytes());
        let mut valid_after_bytes = [0u8; 32];
        valid_after.to_big_endian(&mut valid_after_bytes);
        data.extend_from_slice(&valid_after_bytes[..]);
        let mut valid_before_bytes = [0u8; 32];
        valid_before.to_big_endian(&mut valid_before_bytes);
        data.extend_from_slice(&valid_before_bytes[..]);
        data.extend_from_slice(nonce.as_bytes());

        let mut hasher = Keccak256::new();
        hasher.update([0x19, 0x01]);
        hasher.update(self.domain_separator);
        hasher.update(Keccak256::digest(&data));
        let msg_hash = hasher.finalize();

        msg_hash.into()
    }

    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(err, ret, skip(self, proof))]
    pub async fn burn(
        &self,
        to: &Address,
        proof: &[u8],
        nullifier: &Element,
        value: &Element,
        source: &Element,
        sig: &Element,
    ) -> Result<H256> {
        let to = H160::from_slice(to.as_bytes());

        let call_tx = self
            .call(
                "burn",
                (
                    to,
                    web3::types::Bytes::from(proof),
                    convert_element_to_h256(nullifier),
                    convert_element_to_h256(value),
                    convert_element_to_h256(source),
                    convert_element_to_h256(sig),
                ),
            )
            .await?;

        Ok(call_tx)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn get_mint(&self, key: &Element) -> Result<Option<U256>> {
        let mint: U256 = self
            .client
            .query(
                &self.contract,
                "getMint",
                (convert_element_to_h256(key),),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        if mint == U256::zero() {
            return Ok(None);
        }

        Ok(Some(mint))
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn get_burn(&self, key: &Element) -> Result<Option<Burn>> {
        let burn: Burn = self
            .client
            .query(
                &self.contract,
                "getBurn",
                (convert_element_to_h256(key),),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        if burn.to == H160::zero() && burn.amount == U256::zero() {
            return Ok(None);
        }

        Ok(Some(burn))
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn root_hashes(&self) -> Result<Vec<H256>> {
        let root_hashes = self
            .client
            .query(
                &self.contract,
                "getRootHashes",
                (),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        Ok(root_hashes)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn root_hash(&self) -> Result<H256> {
        let root_hash = self
            .client
            .query(
                &self.contract,
                "currentRootHash",
                (),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        Ok(root_hash)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn block_height(&self) -> Result<u64> {
        let height = self
            .client
            .query(
                &self.contract,
                "blockHeight",
                (),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        Ok(height)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn block_hash(&self) -> Result<H256> {
        let block_hash = self
            .client
            .query(
                &self.contract,
                "blockHash",
                (),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        Ok(block_hash)
    }

    /// Returns all validator sets from a given index, inclusive
    #[tracing::instrument(err, skip(self))]
    pub async fn get_validator_sets(&self, from: u64) -> Result<Vec<ValidatorSet>> {
        let validator_sets = self
            .client
            .query(
                &self.contract,
                "getValidatorSets",
                (U256::from(from),),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        Ok(validator_sets)
    }

    // Listen for ValidatorSetAdded events
    pub async fn listen_for_validator_set_added(
        &self,
        interval: Duration,
    ) -> Result<impl Stream<Item = web3::error::Result<web3::types::Log>>, web3::Error> {
        let filter = FilterBuilder::default()
            .address(vec![self.contract.address()])
            .topics(
                Some(vec![web3::types::H256::from_slice(&Keccak256::digest(
                    "ValidatorSetAdded(uint256,uint256)",
                ))]),
                None,
                None,
                None,
            )
            .build();

        let sub = self
            .client
            .client()
            .eth_filter()
            .create_logs_filter(filter)
            .await?;

        Ok(sub.stream(interval))
    }

    pub fn validators_for_height(&self, height: u64) -> Vec<Address> {
        self
            .validator_sets
            .read()
            .iter()
            .filter(|v| height >= v.valid_from.as_u64())
            .last()
            .expect("No valid validator set found. This should not be possible, unless the contract is uninitialized")
            .validators
            .clone()
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn add_prover(&self, new_prover_address: &Address) -> Result<H256> {
        let call_tx = self
            .call(
                "addProver",
                (web3::types::H160::from_slice(new_prover_address.as_bytes()),),
            )
            .await?;

        Ok(call_tx)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn set_validators(&self, valid_from: u64, addresses: &[Address]) -> Result<H256> {
        let call_tx = self
            .call(
                "setValidators",
                (
                    U256::from(valid_from),
                    Token::Array(addresses.iter().map(|x| Token::Address(*x)).collect()),
                ),
            )
            .await?;

        Ok(call_tx)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn set_root(&self, new_root: &Element) -> Result<H256> {
        let call_tx = self
            .call("setRoot", convert_element_to_h256(new_root))
            .await?;

        Ok(call_tx)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn usdc(&self) -> Result<H160> {
        let usdc = self
            .client
            .query(
                &self.contract,
                "usdc",
                (),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        Ok(usdc)
    }
}
