// lint-long-file-override allow-max-lines=300
use std::io;

use alloy_primitives::{B256 as AlloyB256, Bytes as AlloyBytes};
use alloy_sol_types::{SolCall, SolValue, sol};
use contextful::ErrorContextExt;
use payy_evm_client_interface::{B256, Bytes, Error, Result, TxnData};

use crate::util::keccak32;

sol! {
    contract PrivacyBridge {
        struct TxnData {
            bytes32 verificationKeyHash;
            bytes32[5] senderEncryptedNote;
            bytes32[5] recipientEncryptedNote;
            bytes32[3] senderChainEncryptedKey;
            bytes32[3] recipientChainEncryptedKey;
            bytes32[4] userEncryptedKey;
            bytes32[4] recipientEncryptedKey;
            bytes32 memo;
        }

        function transfer(
            bytes32 verificationKeyHash,
            bytes calldata proof,
            bytes32[] calldata publicInputs,
            bytes32[4] calldata userEncryptedKey,
            bytes32[4] calldata recipientEncryptedKey,
            bytes32 memo
        ) external;
        function burn(
            bytes32 verificationKeyHash,
            bytes calldata proof,
            bytes32[] calldata publicInputs,
            bytes32[4] calldata userEncryptedKey
        ) external;
        function mint(
            bytes32 verificationKeyHash,
            bytes calldata proof,
            bytes32[] calldata publicInputs,
            bytes32[4] calldata userEncryptedKey
        ) external;
        function computeTxHash(
            bytes32 verificationKeyHash,
            bytes calldata proof,
            bytes32[] calldata publicInputs
        ) external view returns (bytes32);
        function getTxnHashByNonceHash(bytes32 nonceHash) external view returns (bytes32);
        function getTxnHashByCommitment(bytes32 commitment) external view returns (bytes32);
        function getTxnData(bytes32 txnHash) external view returns (TxnData memory);
        function getChainPublicKey() external view returns (uint256 x, uint256 y);
        function getMerklePath(bytes32 commitment) external view returns (bytes32 root, bytes32[] memory siblings);
        function getRoot() external view returns (bytes32 root);
        function elementExists(bytes32 element) external view returns (bool);
    }
}

pub(crate) fn encode_get_root_call() -> Bytes {
    PrivacyBridge::getRootCall {}.abi_encode()
}

pub(crate) fn encode_get_merkle_path_call(commitment: B256) -> Bytes {
    PrivacyBridge::getMerklePathCall {
        commitment: alloy_b256(commitment),
    }
    .abi_encode()
}

pub(crate) fn encode_element_exists_call(element: B256) -> Bytes {
    PrivacyBridge::elementExistsCall {
        element: alloy_b256(element),
    }
    .abi_encode()
}

pub(crate) fn encode_get_txn_hash_by_nonce_hash_call(nonce_hash: B256) -> Bytes {
    PrivacyBridge::getTxnHashByNonceHashCall {
        nonceHash: alloy_b256(nonce_hash),
    }
    .abi_encode()
}

pub(crate) fn encode_get_txn_hash_by_commitment_call(commitment: B256) -> Bytes {
    PrivacyBridge::getTxnHashByCommitmentCall {
        commitment: alloy_b256(commitment),
    }
    .abi_encode()
}

pub(crate) fn encode_get_txn_data_call(txn_hash: B256) -> Bytes {
    PrivacyBridge::getTxnDataCall {
        txnHash: alloy_b256(txn_hash),
    }
    .abi_encode()
}

pub(crate) fn encode_get_chain_public_key_call() -> Bytes {
    PrivacyBridge::getChainPublicKeyCall {}.abi_encode()
}

pub(crate) fn encode_mint_call(
    verification_key_hash: B256,
    proof: &[u8],
    public_inputs: &[B256],
    user_encrypted_key: [B256; 4],
) -> Bytes {
    PrivacyBridge::mintCall {
        verificationKeyHash: alloy_b256(verification_key_hash),
        proof: AlloyBytes::from(proof.to_vec()),
        publicInputs: alloy_b256_vec(public_inputs),
        userEncryptedKey: alloy_b256_array_4(user_encrypted_key),
    }
    .abi_encode()
}

pub(crate) fn encode_burn_call(
    verification_key_hash: B256,
    proof: &[u8],
    public_inputs: &[B256],
    user_encrypted_key: [B256; 4],
) -> Bytes {
    PrivacyBridge::burnCall {
        verificationKeyHash: alloy_b256(verification_key_hash),
        proof: AlloyBytes::from(proof.to_vec()),
        publicInputs: alloy_b256_vec(public_inputs),
        userEncryptedKey: alloy_b256_array_4(user_encrypted_key),
    }
    .abi_encode()
}

pub(crate) fn encode_transfer_call(
    verification_key_hash: B256,
    proof: &[u8],
    public_inputs: &[B256],
    user_encrypted_key: [B256; 4],
    recipient_encrypted_key: [B256; 4],
    memo: B256,
) -> Bytes {
    PrivacyBridge::transferCall {
        verificationKeyHash: alloy_b256(verification_key_hash),
        proof: AlloyBytes::from(proof.to_vec()),
        publicInputs: alloy_b256_vec(public_inputs),
        userEncryptedKey: alloy_b256_array_4(user_encrypted_key),
        recipientEncryptedKey: alloy_b256_array_4(recipient_encrypted_key),
        memo: alloy_b256(memo),
    }
    .abi_encode()
}

pub(crate) fn decode_b256(method: &'static str, bytes: &[u8]) -> Result<B256> {
    word_at_exact(method, bytes, 0)
}

pub(crate) fn decode_bool(method: &'static str, bytes: &[u8]) -> Result<bool> {
    let word = word_at_exact(method, bytes, 0)?;
    if word == [0; 32] {
        return Ok(false);
    }
    if word[31] == 1 && word[..31].iter().all(|byte| *byte == 0) {
        return Ok(true);
    }
    Err(malformed(method))
}

pub(crate) fn decode_merkle_path(bytes: &[u8]) -> Result<(B256, Vec<B256>)> {
    let root = word_at_exact("getMerklePath.root", bytes, 0)?;
    let offset = decode_usize_word(
        "getMerklePath.offset",
        word_at_exact("getMerklePath", bytes, 1)?,
    )?;
    if offset % 32 != 0 {
        return Err(malformed("getMerklePath.offset"));
    }
    let len = decode_usize_word(
        "getMerklePath.length",
        word_at_exact("getMerklePath", bytes, offset / 32)?,
    )?;
    let start = offset
        .checked_add(32)
        .ok_or_else(|| malformed("getMerklePath.offset"))?;
    let siblings = (0..len)
        .map(|index| word_at_offset("getMerklePath.sibling", bytes, start + index * 32))
        .collect::<Result<Vec<_>>>()?;
    Ok((root, siblings))
}

pub(crate) fn decode_txn_data(bytes: &[u8]) -> Result<TxnData> {
    let mut index = 0;
    let verification_key_hash = word_at_exact("getTxnData", bytes, index)?;
    index += 1;
    let sender_encrypted_note = read_word_array::<5>(bytes, &mut index)?;
    let recipient_encrypted_note = read_word_array::<5>(bytes, &mut index)?;
    let sender_chain_encrypted_key = read_word_array::<3>(bytes, &mut index)?;
    let recipient_chain_encrypted_key = read_word_array::<3>(bytes, &mut index)?;
    let user_encrypted_key = read_word_array::<4>(bytes, &mut index)?;
    let recipient_encrypted_key = read_word_array::<4>(bytes, &mut index)?;
    let memo = word_at_exact("getTxnData.memo", bytes, index)?;
    Ok(TxnData {
        verification_key_hash,
        sender_encrypted_note,
        recipient_encrypted_note,
        sender_chain_encrypted_key,
        recipient_chain_encrypted_key,
        user_encrypted_key,
        recipient_encrypted_key,
        memo,
    })
}

pub(crate) fn compute_tx_hash(
    verification_key_hash: B256,
    proof: &[u8],
    public_inputs: &[B256],
) -> B256 {
    let encoded = (
        alloy_b256(verification_key_hash),
        AlloyBytes::from(proof.to_vec()),
        alloy_b256_vec(public_inputs),
    )
        .abi_encode_params();
    keccak32(&[&encoded])
}

pub(crate) fn decode_chain_public_key(bytes: &[u8]) -> Result<(B256, B256)> {
    Ok((
        word_at_exact("getChainPublicKey.x", bytes, 0)?,
        word_at_exact("getChainPublicKey.y", bytes, 1)?,
    ))
}

fn read_word_array<const N: usize>(bytes: &[u8], index: &mut usize) -> Result<[B256; N]> {
    let mut words = [[0u8; 32]; N];
    for word in &mut words {
        *word = word_at_exact("getTxnData.array", bytes, *index)?;
        *index += 1;
    }
    Ok(words)
}

fn word_at_exact(method: &'static str, bytes: &[u8], index: usize) -> Result<B256> {
    word_at_offset(method, bytes, index * 32)
}

fn word_at_offset(method: &'static str, bytes: &[u8], offset: usize) -> Result<B256> {
    bytes
        .get(offset..offset + 32)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(|| malformed(method))
}

fn decode_usize_word(method: &'static str, word: B256) -> Result<usize> {
    if word[..24].iter().any(|byte| *byte != 0) {
        return Err(malformed(method));
    }
    let raw = u64::from_be_bytes(word[24..].try_into().map_err(|err| {
        Error::Internal(
            io::Error::other(err)
                .context("decode ABI usize word")
                .into(),
        )
    })?);
    usize::try_from(raw).map_err(|err| Error::Internal(err.context(method).into()))
}

fn malformed(method: &'static str) -> Error {
    Error::Internal(
        io::Error::new(io::ErrorKind::InvalidData, "malformed PrivacyBridge return")
            .context(method)
            .into(),
    )
}

fn alloy_b256(value: B256) -> AlloyB256 {
    AlloyB256::from(value)
}

fn alloy_b256_vec(values: &[B256]) -> Vec<AlloyB256> {
    values.iter().copied().map(AlloyB256::from).collect()
}

fn alloy_b256_array_4(values: [B256; 4]) -> [AlloyB256; 4] {
    values.map(AlloyB256::from)
}
