// lint-long-file-override allow-max-lines=300
use crate::Client;
use crate::error::Result;
use crate::util::calculate_domain_separator;
use ethereum_types::U64;
use sha3::{Digest, Keccak256};
use testutil::eth::EthNode;
use web3::{
    contract::{
        Contract,
        tokens::{Tokenizable, Tokenize},
    },
    ethabi::{self, Token, encode},
    signing::{Key, SecretKey, SecretKeyRef},
    transports::Http,
    types::{Address, FilterBuilder, H256, U256},
};

#[derive(Debug)]
pub struct AcrossWithAuthorizationContract {
    client: Client,
    contract: Contract<Http>,
    signer: SecretKey,
    signer_address: Address,
    domain_separator: H256,
    address: Address,
    /// The ethereum block height used for all contract calls.
    /// If None, the latest block is used.
    block_height: Option<U64>,
}

impl AcrossWithAuthorizationContract {
    pub fn new(
        client: Client,
        contract: Contract<Http>,
        signer: SecretKey,
        address: Address,
        domain_separator: H256,
    ) -> Self {
        let signer_address = Key::address(&SecretKeyRef::new(&signer));

        Self {
            client,
            contract,
            signer,
            signer_address,
            domain_separator,
            address,
            block_height: None,
        }
    }

    pub fn at_height(mut self, block_height: Option<u64>) -> Self {
        self.block_height = block_height.map(|x| x.into());
        self
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub async fn load(
        client: Client,
        chain_id: u128,
        contract_address: &str,
        signer: SecretKey,
    ) -> Result<Self> {
        let contract_json = include_str!(
            "../../../eth/artifacts/contracts/AcrossWithAuthorization.sol/AcrossWithAuthorization.json"
        );
        let contract = client.load_contract_from_str(contract_address, contract_json)?;
        let domain_separator = calculate_domain_separator(
            "AcrossWithAuthorization",
            "1",
            U256::from(chain_id),
            contract_address.parse()?,
        );
        Ok(Self::new(
            client,
            contract,
            signer,
            contract_address.parse()?,
            domain_separator,
        ))
    }

    pub async fn from_eth_node(eth_node: &EthNode, signer: SecretKey) -> Result<Self> {
        let contract_addr = "TODO";

        let client = Client::from_eth_node(eth_node);
        Self::load(client, 1337, contract_addr, signer).await
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
    pub fn signature_for_deposit(
        &self,
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
        depositor: Address,
        recipient: Address,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
        output_amount: U256,
        fee_amount: U256,
        destination_chain_id: U256,
        exclusive_relayer: Address,
        quote_timestamp: u32,
        fill_deadline: u32,
        exclusivity_deadline: u32,
        message: &[u8],
        secret_key: &SecretKey,
    ) -> [u8; 65] {
        let digest = self.signature_msg_digest_for_deposit(
            valid_after,
            valid_before,
            nonce,
            depositor,
            recipient,
            input_token,
            output_token,
            input_amount,
            output_amount,
            fee_amount,
            destination_chain_id,
            exclusive_relayer,
            quote_timestamp,
            fill_deadline,
            exclusivity_deadline,
            message,
        );

        let signature = secp256k1::SECP256K1.sign_ecdsa_recoverable(
            &secp256k1::Message::from_digest(digest),
            &secp256k1::SecretKey::from_slice(&secret_key.secret_bytes()).expect("32 bytes"),
        );
        let (recovery_id, signature_bytes) = signature.serialize_compact();
        let mut final_signature = [0u8; 65];
        final_signature[0..64].copy_from_slice(&signature_bytes[0..64]);
        final_signature[64] = recovery_id.to_i32() as u8;
        final_signature
    }

    #[allow(clippy::too_many_arguments)]
    pub fn signature_msg_digest_for_deposit(
        &self,
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
        depositor: Address,
        recipient: Address,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
        output_amount: U256,
        fee_amount: U256,
        destination_chain_id: U256,
        exclusive_relayer: Address,
        quote_timestamp: u32,
        fill_deadline: u32,
        exclusivity_deadline: u32,
        message: &[u8],
    ) -> [u8; 32] {
        let deposit_v3_with_authorization_typehash = Keccak256::digest(
            b"DepositV3WithAuthorization(uint256 validAfter,uint256 validBefore,bytes32 nonce,address depositor,address recipient,address inputToken,address outputToken,uint256 inputAmount,uint256 outputAmount,uint256 feeAmount,uint256 destinationChainId,address exclusiveRelayer,uint32 quoteTimestamp,uint32 fillDeadline,uint32 exclusivityDeadline,bytes message)"
        );

        let encoded_struct = encode(&[
            Token::FixedBytes(deposit_v3_with_authorization_typehash.to_vec()),
            Token::Uint(valid_after),
            Token::Uint(valid_before),
            Token::FixedBytes(nonce.as_bytes().to_vec()),
            Token::Address(depositor),
            Token::Address(recipient),
            Token::Address(input_token),
            Token::Address(output_token),
            Token::Uint(input_amount),
            Token::Uint(output_amount),
            Token::Uint(fee_amount),
            Token::Uint(destination_chain_id),
            Token::Address(exclusive_relayer),
            Token::Uint(U256::from(quote_timestamp)),
            Token::Uint(U256::from(fill_deadline)),
            Token::Uint(U256::from(exclusivity_deadline)),
            Token::FixedBytes(Keccak256::digest(message).to_vec()),
        ]);

        let struct_hash = Keccak256::digest(&encoded_struct);

        let mut hasher = Keccak256::new();
        hasher.update(b"\x19\x01");
        hasher.update(self.domain_separator);
        hasher.update(struct_hash);
        let msg_hash = hasher.finalize();

        msg_hash.into()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit_v3_with_authorization(
        &self,
        signature_for_receive: &[u8],
        signature_for_deposit: &[u8],
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
        depositor: Address,
        recipient: Address,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
        output_amount: U256,
        fee_amount: U256,
        destination_chain_id: U256,
        exclusive_relayer: Address,
        quote_timestamp: u32,
        fill_deadline: u32,
        exclusivity_deadline: u32,
        message: Vec<u8>,
    ) -> Result<H256> {
        let r = &signature_for_receive[0..32];
        let s = &signature_for_receive[32..64];
        let v = signature_for_receive[64];
        let v = if v < 27 { v + 27 } else { v };

        let r2 = &signature_for_deposit[0..32];
        let s2 = &signature_for_deposit[32..64];
        let v2 = signature_for_deposit[64];
        let v2 = if v2 < 27 { v2 + 27 } else { v2 };

        self.call(
            "depositV3WithAuthorization",
            &[
                web3::types::U256::from(v).into_token(),
                web3::types::H256::from_slice(r).into_token(),
                web3::types::H256::from_slice(s).into_token(),
                web3::types::U256::from(v2).into_token(),
                web3::types::H256::from_slice(r2).into_token(),
                web3::types::H256::from_slice(s2).into_token(),
                valid_after.into_token(),
                valid_before.into_token(),
                nonce.into_token(),
                depositor.into_token(),
                recipient.into_token(),
                input_token.into_token(),
                output_token.into_token(),
                input_amount.into_token(),
                output_amount.into_token(),
                fee_amount.into_token(),
                destination_chain_id.into_token(),
                exclusive_relayer.into_token(),
                quote_timestamp.into_token(),
                fill_deadline.into_token(),
                exclusivity_deadline.into_token(),
                message.into_token(),
            ][..],
        )
        .await
    }

    pub async fn deposit_event_txn(&self, depositor: Address, nonce: H256) -> Result<Option<H256>> {
        let event = self.contract.abi().event("Deposited")?;
        let topic_filter = event.filter(ethabi::RawTopicFilter {
            topic0: ethabi::Topic::This(depositor.into_token()),
            topic1: ethabi::Topic::This(nonce.into_token()),
            topic2: ethabi::Topic::Any,
        })?;

        let filter = FilterBuilder::default()
            .address(vec![self.address])
            .from_block(web3::types::BlockNumber::Earliest)
            .to_block(web3::types::BlockNumber::Latest)
            .topic_filter(topic_filter)
            .build();

        let logs = self.client.logs(filter).await?;

        Ok(logs
            .into_iter()
            .filter_map(|log| log.transaction_hash)
            .next())
    }
}
