use element::Element;
use serde::{Deserialize, Serialize};

// Merge Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivityMergeStage {
    Init(MergeInitData),
    Claim(MergeInitData),
    Success(MergeInitData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MergeInitData {
    pub private_key: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Element>,
}
