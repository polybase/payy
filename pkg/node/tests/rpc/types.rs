use element::Element;
use primitives::{
    block_height::BlockHeight,
    hash::CryptoHash,
    pagination::{OpaqueCursor, OpaqueCursorChoice},
    sig::Signature,
};
use serde::{Deserialize, Serialize};
use zk_primitives::UtxoProof;

#[derive(Debug, PartialEq, Deserialize)]
pub struct Block {
    pub content: BlockContent,
    pub signature: Signature,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct BlockContent {
    pub header: BlockHeader,
    pub state: BlockState,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct BlockHeader {
    pub height: BlockHeight,
    pub last_block_hash: CryptoHash,
    pub epoch_id: u64,
    pub last_final_block_hash: CryptoHash,
    pub approvals: Vec<Signature>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct BlockState {
    pub root_hash: Element,
    pub txns: Vec<TxnWithInfo>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionResp {
    pub height: BlockHeight,
    pub root_hash: Element,
    pub txn_hash: CryptoHash,
}

#[derive(Debug, Deserialize)]
pub struct HeightResp {
    pub height: BlockHeight,
    pub root_hash: Element,
}

#[derive(Debug, Deserialize)]
pub struct MerklePathResponse {
    pub paths: Vec<Vec<Element>>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct ElementResponse {
    pub element: Element,
    pub height: u64,
    pub root_hash: Element,
    pub txn_hash: CryptoHash,
}

#[derive(Debug, Deserialize)]
pub struct ElementsListItem {
    pub element: Element,
    pub height: u64,
    #[expect(dead_code)]
    pub root_hash: Element,
    #[expect(dead_code)]
    pub txn_hash: CryptoHash,
    pub spent: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ListTxnsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<OpaqueCursorChoice<ListTxnsPosition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<ListTxnOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Deserialize)]
pub struct TxnWithInfo {
    pub proof: UtxoProof,
    pub hash: CryptoHash,
    pub index_in_block: u64,
    pub block_height: BlockHeight,
    pub time: u64,
}

#[derive(Debug, Deserialize)]
pub struct ListTransactionsResponse {
    pub txns: Vec<TxnWithInfo>,
    pub cursor: OpaqueCursor<ListTxnsPosition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ListTxnsPosition {
    pub block: BlockHeight,
    pub txn: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ListTxnOrder {
    NewestToOldest,
    OldestToNewest,
}

#[derive(Debug, Deserialize)]
pub struct GetTransactionResponse {
    pub txn: TxnWithInfo,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ListBlocksQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<OpaqueCursorChoice<BlockHeight>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<ListBlocksOrder>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ListBlocksOrder {
    LowestToHighest,
    HighestToLowest,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Deserialize)]
pub struct BlockWithInfo {
    pub block: Block,
    pub hash: CryptoHash,
    pub time: u64,
}

#[derive(Debug, Deserialize)]
pub struct ListBlocksResponse {
    pub blocks: Vec<BlockWithInfo>,
    pub cursor: OpaqueCursor<BlockHeight>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct SmirkElementInfo {
    pub element: Element,
    pub inserted_at_height: u64,
}

pub type GetAllSmirkElementsResponse = Vec<SmirkElementInfo>;

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub last_7_days_txns: Vec<TxnDayStats>,
    pub today_txns: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxnDayStats {
    pub date: chrono::NaiveDate,
    pub count: u64,
}
