use element::Element;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use crate::transaction::{Category, FundingStatus};

use super::{CardStatus, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct RampCardTransaction {
    pub status: CardStatus,
    pub name: String,
    pub desc: Option<String>,
    pub icon: Option<String>,
    pub category: Category,
    pub pending_refund_amount: Option<Element>,
    pub funding_status: Option<FundingStatus>,
    pub funding_due_amount: Option<Element>,
}

impl From<Transaction> for RampCardTransaction {
    fn from(txn: Transaction) -> Self {
        Self {
            status: txn.status.into(),
            name: txn.name.unwrap_or_default(),
            desc: txn.desc,
            icon: txn.icon,
            category: txn.category,
            pending_refund_amount: txn.pending_refund_amount,
            funding_status: txn.funding_status,
            funding_due_amount: txn.funding_due_amount,
        }
    }
}
