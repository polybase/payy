use element::Element;
use serde::{Deserialize, Serialize};

use crate::{PayyData, SnarkWitness, StoredNote};

// Txn Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivityTxnStage {
    Init(WalletTxnInitData),
    Success(WalletTxnSuccessData),
    Failed(WalletTxnFailedData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletTxnInitData {
    pub root: Element,
    pub inputs: Vec<StoredNote>,
    pub outputs: Vec<StoredNote>,
    pub nullifiers: Vec<Element>,
    pub snark: SnarkWitness,
    pub is_self_transfer: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletTxnSuccessData {
    pub root: Element,
    pub inputs: Vec<StoredNote>,
    pub outputs: Vec<StoredNote>,
    pub nullifiers: Vec<Element>,
    pub is_self_transfer: Option<bool>,
    #[serde(default)]
    pub height: u64,
    pub payy: Option<PayyData>,
    pub snark: Option<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletTxnFailedData {
    pub root: Element,
    pub inputs: Vec<StoredNote>,
    pub outputs: Vec<StoredNote>,
    pub nullifiers: Vec<Element>,
    pub is_self_transfer: Option<bool>,
    pub error: String,
    pub snark: Option<()>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_notes: Option<Vec<Element>>,
}
