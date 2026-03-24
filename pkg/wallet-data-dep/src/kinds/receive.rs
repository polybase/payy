use element::Element;
use serde::{Deserialize, Serialize};

use crate::StoredNote;

// Receive Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivityReceiveStage {
    Received(WalletActivityReceiveData),
}

impl WalletActivityReceiveStage {
    pub fn value(&self) -> Element {
        match self {
            WalletActivityReceiveStage::Received(data) => data.note.note.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletActivityReceiveData {
    pub note: StoredNote,
}
