use std::str::FromStr;

use element::Element;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use zk_primitives::bridged_polygon_usdc_note_kind;

use super::{RampStatus, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct RampDepositTransaction {
    pub status: RampStatus,
    pub private_key: Element,
    #[serde(default = "bridged_polygon_usdc_note_kind")]
    pub note_kind: Element,
}

impl From<Transaction> for RampDepositTransaction {
    fn from(txn: Transaction) -> Self {
        Self {
            status: txn.status.into(),
            private_key: Element::from_str(
                txn.private_key
                    .as_ref()
                    .expect("deposit transactions require private key"),
            )
            .expect("private key must be valid element"),
            note_kind: txn.to_note_kind(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct RampDepositLinkTransaction {
    pub status: RampStatus,
    #[serde(default = "bridged_polygon_usdc_note_kind")]
    pub note_kind: Element,
}

impl From<Transaction> for RampDepositLinkTransaction {
    fn from(txn: Transaction) -> Self {
        Self {
            status: txn.status.into(),
            note_kind: txn.to_note_kind(),
        }
    }
}
