use element::Element;
use serde::{Deserialize, Serialize};

use crate::StoredNote;

// SendNote Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivitySendNoteStage {
    Init(WalletActivitySendNoteInitData),
    Txn(WalletActivitySendNoteInitData),
}

impl WalletActivitySendNoteStage {
    pub fn value(&self) -> Element {
        match self {
            WalletActivitySendNoteStage::Init(data) => data.note.note.value,
            WalletActivitySendNoteStage::Txn(data) => data.note.note.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivitySendNoteInitData {
    pub to: Element,
    pub public_key: Element,
    pub private_key: Element,
    pub note: StoredNote,
}
