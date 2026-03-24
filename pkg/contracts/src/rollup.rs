// lint-long-file-override allow-max-lines=1200
use std::sync::Arc;
use std::time::Duration;

use crate::constants::AGG_FINAL_MESSAGES;
use crate::error::Result;
use crate::util::{calculate_domain_separator, convert_element_to_h256};
use crate::{Client, client::retry_on_network_failure};
use element::Element;
use eth_util::Eth;
use ethereum_types::{H160, H256, U64, U256};
use parking_lot::RwLock;
use secp256k1::{Message, SECP256K1};
use sha3::{Digest, Keccak256};
use testutil::eth::EthNode;
use tracing::warn;
use web3::contract::tokens::{Detokenize, Tokenizable, TokenizableItem, Tokenize};
use web3::ethabi::Token;
use web3::futures::{Stream, StreamExt};
use web3::signing::SecretKeyRef;
use web3::transports::Http;
use web3::types::{BlockNumber, FilterBuilder};
use web3::{
    contract::Contract,
    signing::{Key, SecretKey},
    types::Address,
};

pub const AGG_FINAL_VERIFICATION_KEY_HASH: &str =
    "0x122d2ac7542fa020cbfff0836b5d0c30898330074b19869179bba49b5db69967";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatorSet {
    pub validators: Vec<Address>,
    pub valid_from: U256,
}

#[expect(
    dead_code,
    reason = "Helper type for decoding burn events in downstream tooling"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Burn {
    pub to: H256,
    pub amount: U256,
    pub kind: H256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mint {
    pub note_kind: Element,
    pub amount: Element,
    pub spent: bool,
}

impl Tokenize for Mint {
    fn into_tokens(self) -> Vec<Token> {
        vec![
            self.note_kind.to_eth_u256().into_token(),
            self.amount.to_eth_u256().into_token(),
        ]
    }
}

impl Detokenize for Mint {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, web3::contract::Error> {
        // Handle the case where we get a single Tuple token containing the three values
        let (note_kind, amount, spent) = match &tokens[0] {
            Token::Tuple(inner_tokens) => {
                if inner_tokens.len() != 3 {
                    return Err(web3::contract::Error::InvalidOutputType(
                        "expected tuple with 3 elements".to_string(),
                    ));
                }
                (
                    H256::from_token(inner_tokens[0].clone())?,
                    U256::from_token(inner_tokens[1].clone())?,
                    bool::from_token(inner_tokens[2].clone())?,
                )
            }
            _ => {
                return Err(web3::contract::Error::InvalidOutputType(
                    "expected tuple token".to_string(),
                ));
            }
        };

        Ok(Mint {
            note_kind: Element::from_be_bytes(note_kind.0),
            amount: Element::from_u64_array(amount.0),
            spent,
        })
    }
}

/// Represents a MintAdded event from the contract
#[derive(Debug, Clone)]
pub struct MintAddedEvent {
    pub mint_hash: H256,
    pub value: U256,
    pub note_kind: H256,
    pub transaction_hash: H256,
    pub block_number: u64,
}

impl From<(H160, U256)> for Burn {
    fn from(item: (H160, U256)) -> Self {
        Self {
            to: item.0.into(),
            amount: item.1,
            kind: H256::zero(),
        }
    }
}

/// EVM event emitted when a burn occurs (funds are sent from the Rollup contract).
/// Event will be triggered for burns and substituted burns. If a burn is substituted,
/// two events will be emitted. First for the substituted burn, and then again for the refund
/// to the substitutor.
#[derive(Debug)]
pub struct BurnedEvent {
    /// The address of the token being burnt
    pub token: Address,
    /// The burn hash for the burn event
    pub burn_hash: H256,
    /// Whether the burn occurred due to a substitute
    pub substitute: bool,
    /// Recipient of the burn
    pub recipient: Address,
    /// Returns whether the burn was successful, it can be unsuccessful if
    /// IERC20(token).transfer throws an error
    pub success: bool,
    /// The EVM block number this event was emitted in
    pub block_number: Option<u64>,
    /// The EVM txn hash this event was emitted in
    pub txn_hash: Option<H256>,
}

impl From<(H256, U256, H256)> for Burn {
    fn from(item: (H256, U256, H256)) -> Self {
        Self {
            to: item.0,
            amount: item.1,
            kind: item.2,
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
                if tokens.len() != 3 {
                    return Err(web3::contract::Error::InvalidOutputType(
                        "expected tuple of length 3".to_string(),
                    ));
                }

                let mut tokens = tokens.into_iter();
                let (to, amount, kind) = (
                    tokens.next().unwrap(),
                    tokens.next().unwrap(),
                    tokens.next().unwrap(),
                );

                let to = H256::from_token(to)?;
                let amount = U256::from_token(amount)?;
                let kind = H256::from_token(kind)?;

                Ok(Self { to, amount, kind })
            }
            _ => Err(web3::contract::Error::InvalidOutputType(
                "expected tuple".to_string(),
            )),
        }
    }

    fn into_token(self) -> Token {
        Token::Tuple(vec![
            Token::FixedBytes(self.to.to_fixed_bytes().to_vec()),
            Token::Uint(self.amount),
            Token::FixedBytes(self.kind.to_fixed_bytes().to_vec()),
        ])
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

#[derive(Debug, Clone)]
pub struct RollupContract {
    pub client: Client,
    pub contract: Contract<Http>,
    pub signer: SecretKey,
    pub signer_address: Address,
    pub domain_separator: H256,
    pub validator_sets: Arc<RwLock<Vec<ValidatorSet>>>,
    /// Address of rollup contract
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
        chain_id: u128,
        rollup_contract_addr: &str,
        signer: SecretKey,
    ) -> Result<Self> {
        let contract_json =
            include_str!("../../../eth/artifacts/contracts/rollup3/RollupV1.sol/RollupV1.json");
        let contract = client.load_contract_from_str(rollup_contract_addr, contract_json)?;

        let domain_separator = calculate_domain_separator(
            "Rollup",
            "1",
            U256::from(chain_id),
            rollup_contract_addr.parse()?,
        );

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

    pub async fn from_eth_node(eth_node: &EthNode, secret_key: SecretKey) -> Result<Self> {
        let rollup_addr = "dc64a140aa3e981100a9beca4e685f962f0cf6c9";
        let client = Client::from_eth_node(eth_node);
        Self::load(client, 1337, rollup_addr, secret_key).await
    }

    pub fn at_height(self, height: Option<u64>) -> Self {
        Self {
            block_height: height.map(|x| x.into()),
            ..self
        }
    }

    pub fn with_latest_nonce(mut self) -> Self {
        self.client.use_latest_for_nonce = true;
        self
    }

    async fn load_all_validators(&self) -> Result<()> {
        let all_validators = self.get_validator_sets(0).await?;
        *self.validator_sets.write() = all_validators;
        Ok(())
    }

    pub async fn worker(&self, interval: Duration) -> Result<()> {
        let this = self.clone();
        let handle = tokio::spawn(async move {
            let mut events = this.listen_for_validator_set_added(interval).await?.boxed();
            let mut consecutive_transport_error_count = 0;
            const MAX_CONSECUTIVE_TRANSPORT_ERRORS: u64 = 5;

            while let Some(event) = events.next().await {
                let event = match event {
                    Ok(event) => {
                        consecutive_transport_error_count = 0;

                        event
                    }
                    Err(err @ web3::Error::Transport(_)) => {
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

                                    if consecutive_transport_error_count
                                        > MAX_CONSECUTIVE_TRANSPORT_ERRORS
                                    {
                                        return Err(err.into());
                                    }

                                    warn!(
                                        ?err,
                                        consecutive_transport_error_count,
                                        "Received a transport error while trying to create a new event listener. Retrying."
                                    );
                                    continue;
                                }
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
        });

        handle.await?
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
        old_root: &Element,
        new_root: &Element,
        commit_hash: &Element,
        // 6 utxo * 5 messages per utxo
        utxo_messages: &[Element],
        kzg: &[Element],
        other_hash: [u8; 32],
        height: u64,
        signatures: &[&[u8]],
        gas_per_burn_call: u128,
    ) -> Result<H256> {
        // Ensure we have the correct number of UTXO inputs
        assert_eq!(utxo_messages.len(), AGG_FINAL_MESSAGES);

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

        let utxo_messages = utxo_messages.iter().map(convert_element_to_h256);
        let kzg = kzg.iter().map(convert_element_to_h256);

        let mut public_inputs = vec![
            convert_element_to_h256(old_root),
            convert_element_to_h256(new_root),
            convert_element_to_h256(commit_hash),
        ];
        public_inputs.extend(utxo_messages);
        public_inputs.extend(kzg);

        let call_tx = self
            .call(
                "verifyRollup",
                (
                    U256::from(height),
                    AGG_FINAL_VERIFICATION_KEY_HASH
                        .parse::<H256>()
                        .expect("verification key is parsable"),
                    web3::types::Bytes::from(proof),
                    public_inputs,
                    H256::from_slice(&other_hash),
                    Token::Array(signatures),
                ),
            )
            .await?;

        Ok(call_tx)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn mint(
        &self,
        mint_hash: &Element,
        value: &Element,
        note_kind: &Element,
    ) -> Result<H256> {
        let call_tx = self
            .call(
                "mint",
                (
                    convert_element_to_h256(mint_hash),
                    convert_element_to_h256(value),
                    convert_element_to_h256(note_kind),
                ),
            )
            .await?;

        Ok(call_tx)
    }

    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(err, ret, skip(self))]
    pub async fn mint_with_authorization(
        &self,
        mint_hash: &Element,
        value: &Element,
        note_kind: &Element,
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
                    mint_hash.to_h256(),
                    value.to_h256(),
                    note_kind.to_h256(),
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
        note_kind: Element,
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
            note_kind,
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
        note_kind: Element,
        from: Address,
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
    ) -> [u8; 32] {
        let mut data = Vec::new();
        let mint_with_authorization_typehash = Keccak256::digest(
            "MintWithAuthorization(bytes32 commitment,bytes32 value,bytes32 kind,address from,uint256 validAfter,uint256 validBefore,bytes32 nonce)"
                .as_bytes(),
        );
        data.extend_from_slice(&mint_with_authorization_typehash);
        data.extend_from_slice(convert_element_to_h256(&commitment).as_bytes());
        let mut value_bytes = [0u8; 32];
        value.to_big_endian(&mut value_bytes);
        data.extend_from_slice(&value_bytes[..]);
        data.extend_from_slice(convert_element_to_h256(&note_kind).as_bytes());
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

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn get_mint(&self, key: &Element) -> Result<Option<Mint>> {
        let mint: Mint = self
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

        // Check if the mint is "empty" (both fields are zero)
        if mint.note_kind == Element::ZERO && mint.amount == Element::ZERO {
            return Ok(None);
        }

        Ok(Some(mint))
    }

    /// Gets MintAdded events for a specific mint hash
    #[tracing::instrument(err, ret, skip(self))]
    pub async fn get_mint_added_events(
        &self,
        mint_hash: &Element,
        to_block: BlockNumber,
    ) -> Result<Vec<MintAddedEvent>> {
        // Create the event signature hash
        let event_signature = H256::from_slice(
            &Keccak256::digest(b"MintAdded(bytes32,uint256,bytes32)").as_slice()[0..32],
        );

        // Convert the mint_hash Element to H256
        let mint_hash_h256 = convert_element_to_h256(mint_hash);

        // Build the filter
        let filter = FilterBuilder::default()
            .address(vec![self.contract.address()])
            .from_block(BlockNumber::Earliest)
            .to_block(to_block)
            .topics(
                Some(vec![event_signature]), // Event signature
                Some(vec![mint_hash_h256]),  // First indexed parameter (mint_hash)
                None,                        // No third topic
                None,                        // No fourth topic
            )
            .build();

        // Get logs
        let logs = self.client.client().eth().logs(filter).await?;

        // Parse the logs into MintAddedEvent structs
        let mut events = Vec::new();
        for log in logs {
            if log.data.0.len() >= 64
                && log.transaction_hash.is_some()
                && log.block_number.is_some()
            {
                // Extract amount (first parameter, 32 bytes)
                let amount = U256::from_big_endian(&log.data.0[0..32]);

                // Extract note_kind (second parameter, 32 bytes)
                let mut note_kind = [0u8; 32];
                note_kind.copy_from_slice(&log.data.0[32..64]);
                let note_kind = H256::from(note_kind);

                events.push(MintAddedEvent {
                    mint_hash: mint_hash_h256,
                    value: amount,
                    note_kind,
                    transaction_hash: log.transaction_hash.unwrap(),
                    block_number: log.block_number.unwrap().as_u64(),
                });
            }
        }

        Ok(events)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn substitute_burn(
        &self,
        burn_address: &Address,
        note_kind: &Element,
        hash: &Element,
        amount: &Element,
        burn_block_height: u64,
    ) -> Result<H256> {
        let call_tx = self
            .call(
                "substituteBurn",
                (
                    *burn_address,
                    convert_element_to_h256(note_kind),
                    convert_element_to_h256(hash),
                    U256::from_little_endian(&amount.to_le_bytes()),
                    U256::from(burn_block_height),
                ),
            )
            .await?;

        Ok(call_tx)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn was_burn_substituted(
        &self,
        burn_address: &Address,
        note_kind: &Element,
        hash: &Element,
        amount: &Element,
    ) -> Result<bool> {
        let was_substituted = self
            .client
            .query::<bool, _, _, _>(
                &self.contract,
                "wasBurnSubstituted",
                (
                    Token::Address(*burn_address),
                    convert_element_to_h256(note_kind).into_token(),
                    convert_element_to_h256(hash).into_token(),
                    Token::Uint(U256::from_little_endian(&amount.to_le_bytes())),
                ),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        Ok(was_substituted)
    }

    /// Gets a list of emitted Burn events with the given burn hash. There should only
    /// be one successful event.
    #[tracing::instrument(err, ret, skip(self))]
    pub async fn get_burned_events(
        &self,
        burn_hash: &Element,
        block_height: Option<BlockNumber>,
    ) -> Result<Vec<BurnedEvent>> {
        // Create the event signature hash
        let event_signature = H256::from_slice(
            &Keccak256::digest(b"Burned(address,bytes32,address,bool,bool)").as_slice()[0..32],
        );

        let burn_hash_h256 = burn_hash.to_h256();

        // Build the filter
        let filter = FilterBuilder::default()
            .address(vec![self.contract.address()])
            .from_block(BlockNumber::Earliest)
            .to_block(block_height.unwrap_or(BlockNumber::Latest))
            .topics(
                Some(vec![event_signature]), // Event signature
                None,                        // Don't filter by token address
                Some(vec![burn_hash_h256]),  // Second indexed parameter (nullifier)
                None,                        // No fourth topic
            )
            .build();

        // Get logs
        let logs = self.client.client().eth().logs(filter).await?;

        let mut events = Vec::new();
        for log in logs {
            if log.topics.len() >= 4 && log.data.0.len() >= 64 {
                // Extract token address from first topic
                let token = Address::from_slice(&log.topics[1].as_bytes()[12..32]);

                // Extract recipient address from third topic
                let recipient = Address::from_slice(&log.topics[3].as_bytes()[12..32]);

                // Extract substitute (first boolean)
                let substitute = !log.data.0[31..32].iter().all(|&b| b == 0);

                // Extract success (second boolean)
                let success = !log.data.0[63..64].iter().all(|&b| b == 0);

                events.push(BurnedEvent {
                    token,
                    burn_hash: burn_hash_h256,
                    substitute,
                    recipient,
                    success,
                    txn_hash: log.transaction_hash,
                    block_number: log.block_number.map(|u| u.as_u64()),
                });
            }
        }

        Ok(events)
    }

    /// Checks if a burn with the given nullifier was successful
    #[tracing::instrument(err, ret, skip(self))]
    pub async fn was_burn_successful(
        &self,
        burn_hash: &Element,
        to_block: Option<BlockNumber>,
    ) -> Result<bool> {
        let burned_events = self.get_burned_events(burn_hash, to_block).await?;

        // If there are no events, the burn didn't happen
        if burned_events.is_empty() {
            return Ok(false);
        }

        // Check if any of the events indicate a successful burn
        let burn_successful = burned_events.iter().any(|event| event.success);

        Ok(burn_successful)
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
    ) -> Result<impl Stream<Item = web3::error::Result<web3::types::Log>> + use<>, web3::Error>
    {
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

        let sub = retry_on_network_failure({
            let filter = filter.clone();
            move || self.client.client().eth_filter().create_logs_filter(filter)
        })
        .await?;

        Ok(sub.stream(interval))
    }

    pub fn validators_for_height(&self, height: u64) -> Vec<Address> {
        self.validator_sets
            .read()
            .iter()
            .filter(|v| height >= v.valid_from.as_u64())
            .next_back()
            .map(|vs| vs.validators.clone())
            .unwrap_or_else(|| vec![self.signer_address])
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
