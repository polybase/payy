use element::Element;
use serde::{Deserialize, Serialize};

// Swap Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivitySwapStage {
    Init(SwapInitData),
    Success(SwapSuccessData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SwapInitData {
    pub new_primary_key: Element,
    pub value: Element,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SwapSuccessData {
    pub new_primary_key: Option<()>,
    pub value: Element,
}
