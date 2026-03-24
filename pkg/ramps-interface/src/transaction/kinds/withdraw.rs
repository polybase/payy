use std::str::FromStr;

use element::Element;
use ethereum_types::Address;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use zk_primitives::bridged_polygon_usdc_note_kind;

use super::{RampStatus, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct RampWithdrawTransaction {
    pub status: RampStatus,
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    pub evm_address: Address,
    #[serde(default = "bridged_polygon_usdc_note_kind")]
    pub note_kind: Element,
}

impl From<Transaction> for RampWithdrawTransaction {
    fn from(txn: Transaction) -> Self {
        Self {
            status: txn.status.into(),
            evm_address: Address::from_str(
                txn.evm_address
                    .as_ref()
                    .expect("withdraw transactions require evm_address"),
            )
            .expect("evm_address should be valid hex"),
            note_kind: txn.from_note_kind(),
        }
    }
}
