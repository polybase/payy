// lint-long-file-override allow-max-lines=300
mod abi;

use std::sync::Arc;

use payy_evm_client_interface::{B256, Bytes, PayyEvmCallRequest, Result, TxnData};

use crate::client::ClientInner;

/// Raw PrivacyBridge read surface.
#[derive(Clone)]
pub struct BridgeClient {
    inner: Arc<ClientInner>,
}

/// Root and sibling witness returned by `getMerklePath`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerklePathResult {
    /// Root paired with this path by the bridge read.
    pub root: B256,
    /// Sibling words for the requested commitment.
    pub siblings: Vec<B256>,
}

impl BridgeClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    #[must_use]
    pub(crate) fn encode_get_root_call() -> Bytes {
        abi::encode_get_root_call()
    }

    #[must_use]
    pub(crate) fn encode_get_merkle_path_call(commitment: B256) -> Bytes {
        abi::encode_get_merkle_path_call(commitment)
    }

    #[must_use]
    pub(crate) fn encode_element_exists_call(element: B256) -> Bytes {
        abi::encode_element_exists_call(element)
    }

    #[must_use]
    pub(crate) fn encode_get_txn_hash_by_nonce_hash_call(nonce_hash: B256) -> Bytes {
        abi::encode_get_txn_hash_by_nonce_hash_call(nonce_hash)
    }

    #[must_use]
    pub(crate) fn encode_get_txn_hash_by_commitment_call(commitment: B256) -> Bytes {
        abi::encode_get_txn_hash_by_commitment_call(commitment)
    }

    #[must_use]
    pub(crate) fn encode_get_txn_data_call(txn_hash: B256) -> Bytes {
        abi::encode_get_txn_data_call(txn_hash)
    }

    #[must_use]
    pub(crate) fn encode_get_chain_public_key_call() -> Bytes {
        abi::encode_get_chain_public_key_call()
    }

    /// Read current root.
    pub async fn get_root(&self) -> Result<B256> {
        let bytes = self.read(Self::encode_get_root_call()).await?;
        abi::decode_b256("getRoot", &bytes)
    }

    /// Read Merkle path and the root it was generated against.
    pub async fn get_merkle_path(&self, commitment: B256) -> Result<MerklePathResult> {
        let bytes = self
            .read(Self::encode_get_merkle_path_call(commitment))
            .await?;
        let (root, siblings) = abi::decode_merkle_path(&bytes)?;
        Ok(MerklePathResult { root, siblings })
    }

    /// Check element existence.
    pub async fn element_exists(&self, element: B256) -> Result<bool> {
        let bytes = self.read(Self::encode_element_exists_call(element)).await?;
        abi::decode_bool("elementExists", &bytes)
    }

    /// Resolve txn hash by nonce hash.
    pub async fn get_txn_hash_by_nonce_hash(&self, nonce_hash: B256) -> Result<Option<B256>> {
        self.read_optional_b256(Self::encode_get_txn_hash_by_nonce_hash_call(nonce_hash))
            .await
    }

    /// Resolve txn hash by commitment.
    pub async fn get_txn_hash_by_commitment(&self, commitment: B256) -> Result<Option<B256>> {
        self.read_optional_b256(Self::encode_get_txn_hash_by_commitment_call(commitment))
            .await
    }

    /// Resolve txn data.
    pub async fn get_txn_data(&self, txn_hash: B256) -> Result<TxnData> {
        let bytes = self.read(Self::encode_get_txn_data_call(txn_hash)).await?;
        abi::decode_txn_data(&bytes)
    }

    /// Compute bridge transaction hash.
    #[must_use]
    pub fn compute_tx_hash(
        &self,
        verification_key_hash: B256,
        proof: &[u8],
        public_inputs: &[B256],
    ) -> B256 {
        abi::compute_tx_hash(verification_key_hash, proof, public_inputs)
    }

    /// Read chain public key.
    pub async fn get_chain_public_key(&self) -> Result<(B256, B256)> {
        let bytes = self.read(Self::encode_get_chain_public_key_call()).await?;
        abi::decode_chain_public_key(&bytes)
    }

    #[must_use]
    pub(crate) fn encode_mint_call(
        verification_key_hash: B256,
        proof: &[u8],
        public_inputs: &[B256],
        user_encrypted_key: [B256; 4],
    ) -> Bytes {
        abi::encode_mint_call(
            verification_key_hash,
            proof,
            public_inputs,
            user_encrypted_key,
        )
    }

    #[must_use]
    pub(crate) fn encode_burn_call(
        verification_key_hash: B256,
        proof: &[u8],
        public_inputs: &[B256],
        user_encrypted_key: [B256; 4],
    ) -> Bytes {
        abi::encode_burn_call(
            verification_key_hash,
            proof,
            public_inputs,
            user_encrypted_key,
        )
    }

    #[must_use]
    pub(crate) fn encode_transfer_call(
        verification_key_hash: B256,
        proof: &[u8],
        public_inputs: &[B256],
        user_encrypted_key: [B256; 4],
        recipient_encrypted_key: [B256; 4],
        memo: B256,
    ) -> Bytes {
        abi::encode_transfer_call(
            verification_key_hash,
            proof,
            public_inputs,
            user_encrypted_key,
            recipient_encrypted_key,
            memo,
        )
    }

    async fn read(&self, data: Bytes) -> Result<Bytes> {
        self.inner.validate_read_chain().await?;
        self.inner
            .read_client
            .read_contract(PayyEvmCallRequest {
                to: self.inner.network.privacy_bridge,
                data,
                block_number: None,
            })
            .await
    }

    async fn read_optional_b256(&self, data: Bytes) -> Result<Option<B256>> {
        let bytes = self.read(data).await?;
        let value = abi::decode_b256("bytes32 lookup", &bytes)?;
        Ok((value != [0; 32]).then_some(value))
    }
}
