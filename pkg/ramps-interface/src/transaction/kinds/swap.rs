use element::Element;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use super::{Status, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapStatus {
    Pending,
    Funded,
    Complete,
    Failed,
}

impl From<Status> for SwapStatus {
    fn from(status: Status) -> Self {
        match status {
            Status::Pending => Self::Pending,
            Status::Funded => Self::Funded,
            Status::Complete => Self::Complete,
            Status::Failed => Self::Failed,
            other => panic!("unexpected status {other:?} for swap txn"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct RampSwapTransaction {
    pub status: SwapStatus,
    pub from_note_kind: Element,
    pub to_note_kind: Element,
}

impl From<Transaction> for RampSwapTransaction {
    fn from(txn: Transaction) -> Self {
        Self {
            status: txn.status.into(),
            from_note_kind: txn.from_note_kind(),
            to_note_kind: txn.to_note_kind(),
        }
    }
}
