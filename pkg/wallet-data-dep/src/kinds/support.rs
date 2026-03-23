use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Support Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivitySupportStage {
    Init(SupportInitData),
    Success(SupportInitData),
}

impl WalletActivitySupportStage {
    pub fn issue_id(&self) -> Uuid {
        match self {
            Self::Init(data) => data.issue_id,
            Self::Success(data) => data.issue_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SupportInitData {
    pub issue_id: Uuid,
    pub message: String,
}
