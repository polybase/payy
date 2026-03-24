use element::Element;
use serde::{Deserialize, Serialize};

use crate::StoredNote;

// Claim Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityClaim {
    #[serde(flatten)]
    pub stage: WalletActivityClaimStage,
    pub error_reason: Option<WalletActivityClaimErrorReason>,
}

impl WalletActivityClaim {
    pub fn value(&self) -> Element {
        match &self.stage {
            WalletActivityClaimStage::Init(data) => data.note.note.value,
            WalletActivityClaimStage::Txn(data) => data.note.note.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivityClaimStage {
    Init(WalletActivityClaimInitData),
    Txn(WalletActivityClaimInitData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WalletActivityClaimErrorReason {
    NullifierConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityClaimInitData {
    pub private_key: Element,
    pub note: StoredNote,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    pub source: Option<WalletActivityClaimSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WalletActivityClaimSource {
    Receive,
    Link,
    Cancel,
    Merge,
}
