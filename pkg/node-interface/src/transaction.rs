use element::Element;
use primitives::{
    block_height::BlockHeight,
    pagination::{OpaqueCursor, OpaqueCursorChoice},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use zk_primitives::UtxoProof;

use crate::TxnWithInfo;

/// Request for submit transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct TransactionRequest {
    /// Utxo proof to be verified and applied
    pub proof: UtxoProof,
}

/// Response for submit transaction
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct TransactionResponse {
    /// Height of the block the transaction was included in
    pub height: BlockHeight,
    /// Root hash of the merkle tree for the block
    pub root_hash: Element,
    /// Transaction hash of submitted transaction
    pub txn_hash: Element,
}

/// Ordering options for listing transactions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum ListTxnsOrder {
    /// Return transactions from newest to oldest.
    NewestToOldest,
    /// Return transactions from oldest to newest.
    OldestToNewest,
}

impl ListTxnsOrder {
    /// Convenience helper for newest to oldest order.
    #[must_use]
    pub const fn newest_to_oldest() -> Self {
        Self::NewestToOldest
    }

    /// Convenience helper for oldest to newest order.
    #[must_use]
    pub const fn oldest_to_newest() -> Self {
        Self::OldestToNewest
    }
}

/// Cursor position for listing transactions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct ListTxnsPosition {
    /// Block height for the transaction.
    pub block: BlockHeight,
    /// Transaction index within the block.
    pub txn: u64,
}

/// Parameters for listing transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct ListTxnsParams {
    /// Maximum number of transactions to return.
    pub limit: usize,
    /// Pagination cursor for fetching additional transactions.
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<OpaqueCursorChoice<ListTxnsPosition>>,
    /// Ordering for the listed transactions.
    pub order: ListTxnsOrder,
    /// Whether to wait for a new transaction when none are available.
    pub poll: bool,
}

/// Response structure for listing transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct ListTxnsResponse {
    /// Transactions returned by the node.
    pub txns: Vec<TxnWithInfo>,
    /// Pagination cursor for fetching additional transactions.
    #[cfg_attr(
        feature = "ts-rs",
        ts(type = "import(\"./OpaqueClientCursor\").OpaqueClientCursor")
    )]
    pub cursor: OpaqueCursor<ListTxnsPosition>,
}
