use primitives::{block_height::BlockHeight, hash::CryptoHash};
use wire_message::wire_message;

#[derive(Debug, Clone, PartialEq, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct ElementHistoryData {
    pub block_hash: CryptoHash,
    pub block_height: BlockHeight,
}

#[derive(Debug, Clone, PartialEq)]
#[wire_message(version = 1)]
pub enum ElementHistoryValue {
    V1(ElementHistoryData),
}

#[derive(Debug, Clone, PartialEq, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct MintHashData {
    pub block_hash: CryptoHash,
    pub block_height: BlockHeight,
}

#[derive(Debug, Clone, PartialEq)]
#[wire_message(version = 1)]
pub enum MintHashValue {
    V1(MintHashData),
}
