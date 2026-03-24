use element::Element;
use primitives::{
    block_height::BlockHeight, hash::CryptoHash, pagination::OpaqueCursor, sig::Signature,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use zk_primitives::UtxoProof;

/// Ordering options for listing blocks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum ListBlocksOrder {
    /// Return blocks from the lowest height to the highest.
    LowestToHighest,
    /// Return blocks from the highest height to the lowest.
    HighestToLowest,
}

impl ListBlocksOrder {
    /// Convenience helper for the lowest to highest order.
    #[must_use]
    pub const fn lowest_to_highest() -> Self {
        Self::LowestToHighest
    }

    /// Convenience helper for the highest to lowest order.
    #[must_use]
    pub const fn highest_to_lowest() -> Self {
        Self::HighestToLowest
    }
}

/// Response structure for listing blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct ListBlocksResponse {
    /// Blocks returned by the node.
    pub blocks: Vec<BlockWithInfo>,
    /// Pagination cursor for fetching additional blocks.
    #[cfg_attr(
        feature = "ts-rs",
        ts(type = "import(\"./OpaqueClientCursor\").OpaqueClientCursor")
    )]
    pub cursor: OpaqueCursor<BlockHeight>,
}

/// Block bundle with metadata returned by the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct BlockWithInfo {
    /// Block payload.
    pub block: Block,
    /// Hash of the block.
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    pub hash: CryptoHash,
    /// Timestamp provided by the node.
    pub time: u64,
}

/// High-level block representation exposed over HTTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct Block {
    /// Content of the block.
    pub content: BlockContent,
    /// Validator signature for the block.
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    pub signature: Signature,
}

/// Block content containing the header and state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct BlockContent {
    /// Block header metadata.
    pub header: BlockHeader,
    /// Block state describing the transactions.
    pub state: BlockState,
}

/// Header information for a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct BlockHeader {
    /// Height of the block.
    pub height: BlockHeight,
    /// Hash of the previous block.
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    pub last_block_hash: CryptoHash,
    /// Epoch identifier.
    pub epoch_id: u64,
    /// Hash of the last finalized block.
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    pub last_final_block_hash: CryptoHash,
    /// Approvals collected for the block.
    #[cfg_attr(feature = "ts-rs", ts(type = "string[]"))]
    pub approvals: Vec<Signature>,
}

/// State payload for the block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct BlockState {
    /// Merkle root hash after executing this block.
    pub root_hash: Element,
    /// Transactions included in the block.
    pub txns: Vec<TxnWithInfo>,
}

/// Transaction data with additional metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct TxnWithInfo {
    /// Transaction proof.
    pub proof: UtxoProof,
    /// Hash of the transaction.
    pub hash: Element,
    /// Index of the transaction inside the block.
    pub index_in_block: u64,
    /// Height of the block that included the transaction.
    pub block_height: BlockHeight,
    /// Timestamp assigned to the transaction.
    pub time: u64,
}
