use chrono::{DateTime, Utc};
use currency::Currency;
use element::Element;
use network::{Network, NetworkIdentifier};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use uuid::Uuid;

use crate::provider::Provider;

use super::{Status, Transaction, TransactionKind};

mod card;
mod deposit;
mod withdraw;

pub use card::*;
pub use deposit::*;
pub use withdraw::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RampTransaction {
    #[serde(flatten)]
    pub base: RampTransactionBase,
    #[serde(flatten)]
    pub kind: RampTransactionKind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "kind")]
pub enum RampTransactionKind {
    Card(RampCardTransaction),
    Withdraw(RampWithdrawTransaction),
    Deposit(RampDepositTransaction),
    DepositLink(RampDepositLinkTransaction),
}

impl From<Transaction> for RampTransaction {
    fn from(txn: Transaction) -> Self {
        let kind = match txn.kind() {
            TransactionKind::Card => RampTransactionKind::Card(txn.clone().into()),
            TransactionKind::Withdraw => RampTransactionKind::Withdraw(txn.clone().into()),
            TransactionKind::Deposit => RampTransactionKind::Deposit(txn.clone().into()),
            TransactionKind::DepositLink => RampTransactionKind::DepositLink(txn.clone().into()),
        };

        Self {
            base: RampTransactionBase::from(txn),
            kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct RampTransactionBase {
    pub id: Uuid,
    pub account_id: Uuid,
    pub provider: Provider,
    pub from_currency: Currency,
    pub from_amount: Element,
    pub from_network: Network,
    pub from_network_identifier: Option<NetworkIdentifier>,
    pub to_currency: Currency,
    pub to_amount: Element,
    pub to_network: Network,
    pub to_network_identifier: Option<NetworkIdentifier>,
    pub updated_at: DateTime<Utc>,
    pub added_at: DateTime<Utc>,
}

impl From<Transaction> for RampTransactionBase {
    fn from(txn: Transaction) -> Self {
        Self {
            id: txn.id,
            account_id: txn.account_id,
            provider: txn.provider,
            from_currency: txn.from_currency,
            from_amount: txn.from_amount,
            from_network: txn.from_network,
            from_network_identifier: txn.from_network_identifier,
            to_currency: txn.to_currency,
            to_amount: txn.to_amount,
            to_network: txn.to_network,
            to_network_identifier: txn.to_network_identifier,
            updated_at: txn.updated_at,
            added_at: txn.added_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CardStatus {
    Pending,
    Complete,
    Declined,
    Refunded,
}

impl From<Status> for CardStatus {
    fn from(status: Status) -> Self {
        match status {
            Status::Pending => CardStatus::Pending,
            Status::Complete => CardStatus::Complete,
            Status::Declined => CardStatus::Declined,
            Status::Refunded => CardStatus::Refunded,
            other => panic!("unexpected status {other:?} for card txn"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RampStatus {
    Pending,
    Complete,
    Cancelled,
    Refunded,
    Funded,
    Withdraw,
}

impl From<Status> for RampStatus {
    fn from(status: Status) -> Self {
        match status {
            Status::Pending => RampStatus::Pending,
            Status::Complete => RampStatus::Complete,
            Status::Cancelled => RampStatus::Cancelled,
            Status::Refunded => RampStatus::Refunded,
            Status::Funded => RampStatus::Funded,
            Status::Withdraw => RampStatus::Withdraw,
            other => panic!("unexpected status {other:?} for ramp txn"),
        }
    }
}
