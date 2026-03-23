// lint-long-file-override allow-max-lines=300
use crate::Client;
use crate::error::Result;
use crate::util::calculate_domain_separator;
use ethereum_types::U64;
use rustc_hex::FromHex;
use secp256k1::{Message, SECP256K1};
use sha3::{Digest, Keccak256};
use testutil::eth::EthNode;
use web3::{
    contract::{Contract, tokens::Tokenize},
    signing::{Key, SecretKey, SecretKeyRef},
    transports::Http,
    types::{Address, FilterBuilder, H256, U256},
};

#[derive(Debug, Clone)]
pub struct USDCContract {
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

impl USDCContract {
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
            address,
            block_height: None,
        }
    }

    pub fn at_height(&self, block_height: u64) -> Self {
        Self {
            block_height: Some(U64::from(block_height)),
            ..self.clone()
        }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub async fn load(
        client: Client,
        chain_id: u128,
        usdc_contract_addr: &str,
        signer: SecretKey,
    ) -> Result<Self> {
        let contract_json = include_str!("../../../eth/artifacts/contracts/IUSDC.sol/IUSDC.json");
        let contract = client.load_contract_from_str(usdc_contract_addr, contract_json)?;
        let domain_separator = calculate_domain_separator(
            "USD Coin",
            "2",
            U256::from(chain_id),
            usdc_contract_addr.parse()?,
        );
        Ok(Self::new(
            client,
            contract,
            signer,
            domain_separator,
            usdc_contract_addr.parse()?,
        ))
    }

    pub async fn from_eth_node(eth_node: &EthNode, signer: SecretKey) -> Result<Self> {
        let usdc_addr = "5fbdb2315678afecb367f032d93f642f64180aa3";

        let client = Client::from_eth_node(eth_node);
        Self::load(client, 1337, usdc_addr, signer).await
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
    pub fn signature_for_receive(
        &self,
        from: Address,
        to: Address,
        amount: U256,
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
        signer: secp256k1::SecretKey,
    ) -> [u8; 65] {
        let msg_digest = Self::signature_msg_digest_for_receive(
            self.domain_separator,
            from,
            to,
            amount,
            valid_after,
            valid_before,
            nonce,
        );

        // Sig for the USDC's receiveWithAuthorization
        let signature =
            SECP256K1.sign_ecdsa_recoverable(&Message::from_digest(msg_digest), &signer);

        let (recovery_id, signature) = signature.serialize_compact();
        let mut sig_bytes = [0u8; 65];
        sig_bytes[0..64].copy_from_slice(&signature[0..64]);
        sig_bytes[64] = recovery_id.to_i32() as u8;
        sig_bytes
    }

    /// Prepares signature message digest for `receiveWithAuthorization`.
    /// The digest is computed as follows:
    /// ```no_compile
    /// keccak256(
    ///     b'\x19\x01',
    ///     DOMAIN_SEPARATOR,
    ///     keccak256(
    ///         abi.encode(
    ///             RECEIVE_WITH_AUTHORIZATION_TYPEHASH,
    ///             from,
    ///             to,
    ///             value,
    ///             validAfter,
    ///             validBefore,
    ///             nonce
    ///         )
    ///     )
    /// )
    /// ```
    pub fn signature_msg_digest_for_receive(
        domain_separator: H256,
        from: Address,
        to: Address,
        amount: U256,
        valid_after: U256,
        valid_before: U256,
        nonce: H256,
    ) -> [u8; 32] {
        let mut data = Vec::new();
        // keccak256("ReceiveWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)")
        let receive_with_authorization_typehash =
            "d099cc98ef71107a616c4f0f941f04c322d8e254fe26b3c6668db87aae413de8"
                .from_hex::<Vec<_>>()
                .unwrap();
        data.extend_from_slice(&receive_with_authorization_typehash);
        data.extend_from_slice(H256::from(from).as_bytes());
        data.extend_from_slice(H256::from(to).as_bytes());
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);
        let mut valid_after_bytes = [0u8; 32];
        valid_after.to_big_endian(&mut valid_after_bytes);
        data.extend_from_slice(&valid_after_bytes);
        let mut valid_before_bytes = [0u8; 32];
        valid_before.to_big_endian(&mut valid_before_bytes);
        data.extend_from_slice(&valid_before_bytes);
        data.extend_from_slice(nonce.as_bytes());

        let mut hasher = Keccak256::new();
        hasher.update([0x19, 0x01]);
        hasher.update(domain_separator);
        hasher.update(Keccak256::digest(&data));
        let msg_hash = hasher.finalize();

        msg_hash.into()
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn mint(&self, to: Address, amount: u128) -> Result<H256> {
        self.call("mint", (to, amount)).await
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn transfer(&self, to: Address, amount: u128) -> Result<H256> {
        self.call("transfer", (to, amount)).await
    }

    // Query allowance
    #[tracing::instrument(err, ret, skip(self))]
    pub async fn allowance(&self, owner: Address, spender: Address) -> Result<U256> {
        let allowance = self
            .client
            .query(
                &self.contract,
                "allowance",
                (owner, spender),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;

        Ok(allowance)
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn balance(&self, owner: Address) -> Result<U256> {
        let balance = self
            .client
            .query(
                &self.contract,
                "balanceOf",
                (owner,),
                None,
                Default::default(),
                self.block_height.map(|x| x.into()),
            )
            .await?;
        Ok(balance)
    }

    /// Check if an EIP-3009 authorization has been used by scanning the AuthorizationUsed event.
    /// Many USDC implementations emit: AuthorizationUsed(address indexed authorizer, bytes32 indexed nonce)
    /// Returns the transaction hash if found.
    #[tracing::instrument(err, ret, skip(self))]
    pub async fn authorization_used_txn(
        &self,
        authorizer: Address,
        nonce: H256,
    ) -> Result<Option<H256>> {
        // keccak256("AuthorizationUsed(address,bytes32)")
        let topic0 = H256::from_slice(&Keccak256::digest(b"AuthorizationUsed(address,bytes32)"));
        let authorizer_h = H256::from(authorizer);

        let filter = FilterBuilder::default()
            .address(vec![self.address])
            .from_block(web3::types::BlockNumber::Earliest)
            .to_block(web3::types::BlockNumber::Latest)
            .topics(
                Some(vec![topic0]),
                Some(vec![authorizer_h]),
                Some(vec![nonce]),
                None,
            )
            .build();

        let logs = self.client.logs(filter).await?;
        Ok(logs.into_iter().filter_map(|l| l.transaction_hash).next())
    }

    /// Approve contract to spend USDC on behalf of the user
    #[tracing::instrument(err, ret, skip(self))]
    pub async fn approve_max(&self, from: Address) -> Result<H256> {
        self.call("approve", (from, web3::types::U256::MAX)).await
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn approve(&self, from: Address, amount: u128) -> Result<H256> {
        self.call("approve", (from, amount)).await
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::tests::get_env;

//     // TODO: fix this test
//     #[tokio::test]
//     async fn test_approve() {
//         let env = get_env();
//         let allowance = env
//             .usdc_contract
//             .allowance(env.evm_address, env.rollup_contract_addr)
//             .await
//             .unwrap();

//         assert_eq!(allowance, U256::max_value());
//     }
// }
