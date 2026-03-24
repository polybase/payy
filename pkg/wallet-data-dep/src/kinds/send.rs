use element::Element;
use serde::{Deserialize, Serialize};
use wallet_primitives::derive_private_key;

use crate::{PayyData, StoredNote};

// Send Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivitySendStage {
    Init(SendInitData),
    Txn(SendTxnCombinedData),
    Failed(SendFailedCombinedData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendInitData {
    #[serde(rename = "type")]
    pub send_type: Option<WalletActivitySendType>,
    pub to: Element,
    pub value: Element,
    pub public_key: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_version: Option<u32>,
}

impl WalletActivitySendStage {
    pub fn init_data(&self) -> SendInitData {
        match self {
            WalletActivitySendStage::Init(init) => init.clone(),
            WalletActivitySendStage::Txn(txn) => txn.init_data.clone(),
            WalletActivitySendStage::Failed(failed) => failed.init_data.clone(),
        }
    }

    pub fn note(&self) -> Option<StoredNote> {
        let private_key = self.init_data().private_key?;
        let psi = match &self {
            WalletActivitySendStage::Init(_) => None,
            WalletActivitySendStage::Txn(txn) => txn.txn_data.note.as_ref().map(|n| n.note.psi),
            WalletActivitySendStage::Failed(txn) => txn.txn_data.note.as_ref().map(|n| n.note.psi),
        };
        Some(StoredNote::claimable_from_private_key_psi(
            self.value(),
            private_key,
            psi.unwrap_or(derive_private_key(&[], private_key)),
        ))
    }

    pub fn value(&self) -> Element {
        match self {
            WalletActivitySendStage::Init(init) => init.value,
            WalletActivitySendStage::Txn(txn) => txn.init_data.value,
            WalletActivitySendStage::Failed(failed) => failed.init_data.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WalletActivitySendType {
    Claimable,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendTxnData {
    pub height: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<StoredNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_spent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payy: Option<PayyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendTxnCombinedData {
    #[serde(flatten)]
    pub init_data: SendInitData,
    #[serde(flatten)]
    pub txn_data: SendTxnData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendFailedCombinedData {
    #[serde(flatten)]
    pub init_data: SendInitData,
    #[serde(flatten)]
    pub txn_data: SendTxnData,
    pub error: String,
}
