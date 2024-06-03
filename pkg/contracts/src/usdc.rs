use crate::error::Result;
use crate::Client;
use ethereum_types::U64;
use rustc_hex::FromHex;
use secp256k1::{Message, SECP256K1};
use sha3::{Digest, Keccak256};
use web3::{
    contract::{tokens::Tokenize, Contract},
    signing::{Key, SecretKey, SecretKeyRef},
    transports::Http,
    types::{Address, H256, U256},
};

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

    pub fn at_height(mut self, block_height: Option<u64>) -> Self {
        self.block_height = block_height.map(|x| x.into());
        self
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub async fn load(client: Client, usdc_contract_addr: &str, signer: SecretKey) -> Result<Self> {
        let contract_json = include_str!("../../../eth/artifacts/contracts/IUSDC.sol/IUSDC.json");
        let contract = client.load_contract_from_str(usdc_contract_addr, contract_json)?;
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
        Ok(Self::new(
            client,
            contract,
            signer,
            domain_separator,
            usdc_contract_addr.parse()?,
        ))
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
        let msg_digest = self.signature_msg_digest_for_receive(
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
        &self,
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
        hasher.update(self.domain_separator);
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

    /// Approve contract to spend USDC on behalf of the user
    #[tracing::instrument(err, ret, skip(self))]
    pub async fn approve_max(&self, from: Address) -> Result<H256> {
        self.call("approve", (from, web3::types::U256::MAX)).await
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
