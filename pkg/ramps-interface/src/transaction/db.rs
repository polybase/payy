// lint-long-file-override allow-max-lines=300

use chrono::{DateTime, Utc};
use currency::Currency;
#[cfg(feature = "diesel")]
use database::schema::ramps_transactions;
use element::Element;
use network::{Network, NetworkIdentifier};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{Display, EnumString};
use uuid::Uuid;
use zk_primitives::bridged_polygon_usdc_note_kind;

#[cfg(feature = "diesel")]
use crate::derive_pg_text_enum;
use crate::provider::Provider;

use super::{Category, FundingStatus, Status, TransactionStatusReason};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    feature = "diesel",
    derive(
        diesel::Queryable,
        diesel::Selectable,
        diesel::Identifiable,
        diesel::Insertable,
        diesel::AsChangeset
    )
)]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = ramps_transactions))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Transaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub quote_id: Option<Uuid>,
    pub account_id: Uuid,
    pub provider: Provider,
    pub external_id: Option<String>,
    pub external_fund_id: Option<String>,
    pub status: Status,
    pub funding_status: Option<FundingStatus>,
    pub funding_kind: FundingKind,
    pub status_reason: Option<TransactionStatusReason>,
    pub from_currency: Currency,
    pub from_amount: Element,
    pub from_network: Network,
    pub from_network_identifier: Option<NetworkIdentifier>,
    pub pending_refund_amount: Option<Element>,
    pub funding_due_amount: Option<Element>,
    pub from_note_kind: Option<Element>,
    pub to_note_kind: Option<Element>,
    pub to_currency: Currency,
    pub to_amount: Element,
    pub to_network: Network,
    pub to_network_identifier: Option<NetworkIdentifier>,
    pub evm_address: Option<String>,
    pub private_key: Option<String>,
    pub name: Option<String>,
    pub memo: Option<String>,
    pub desc: Option<String>,
    pub emoji: Option<String>,
    pub icon: Option<String>,
    pub category: Category,
    pub metadata: Option<Value>,
    pub transaction_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub added_at: DateTime<Utc>,
}

impl Default for Transaction {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::nil(),
            wallet_id: Uuid::nil(),
            quote_id: None,
            account_id: Uuid::nil(),
            provider: Provider::Alfred,
            external_id: None,
            external_fund_id: None,
            status: Status::Pending,
            funding_status: None,
            funding_kind: FundingKind::Crypto,
            status_reason: None,
            from_currency: Currency::USD,
            from_amount: Element::default(),
            from_network: Network::Payy,
            from_network_identifier: None,
            pending_refund_amount: None,
            funding_due_amount: None,
            from_note_kind: None,
            to_note_kind: None,
            to_currency: Currency::USD,
            to_amount: Element::default(),
            to_network: Network::Payy,
            to_network_identifier: None,
            evm_address: None,
            private_key: None,
            name: None,
            memo: None,
            desc: None,
            emoji: None,
            icon: None,
            category: Category::Transfer,
            metadata: None,
            transaction_at: None,
            updated_at: now,
            added_at: now,
        }
    }
}

pub enum TransactionKind {
    Deposit,
    DepositLink,
    Withdraw,
    Card,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionAction {
    Deposit,
    Withdraw,
}

#[derive(
    Debug, Default, EnumString, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FundingKind {
    #[default]
    Crypto,
    Link,
    UserRemoteNotes,
}

#[cfg(feature = "diesel")]
derive_pg_text_enum!(FundingKind, "SCREAMING_SNAKE_CASE");

impl Transaction {
    #[must_use]
    pub fn from_note_kind(&self) -> Element {
        self.from_note_kind
            .unwrap_or_else(bridged_polygon_usdc_note_kind)
    }

    #[must_use]
    pub fn to_note_kind(&self) -> Element {
        self.to_note_kind
            .unwrap_or_else(bridged_polygon_usdc_note_kind)
    }

    #[must_use]
    pub fn kind(&self) -> TransactionKind {
        match (self.from_network, self.to_network) {
            (Network::Card, _) | (_, Network::Card) => TransactionKind::Card,
            (Network::Payy, _) => TransactionKind::Withdraw,
            (_, Network::Payy) => match self.funding_kind {
                FundingKind::Crypto => TransactionKind::Deposit,
                FundingKind::Link => TransactionKind::DepositLink,
                FundingKind::UserRemoteNotes => {
                    unreachable!("invalid funding kind UserRemoteNotes for deposit")
                }
            },
            _ => unreachable!("one of to_network, from_network must be Network::Payy"),
        }
    }

    #[must_use]
    pub fn is_onramp(&self) -> bool {
        self.to_network == Network::Payy
    }

    #[must_use]
    pub fn is_offramp(&self) -> bool {
        self.from_network == Network::Payy
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "diesel", derive(diesel::Queryable, diesel::AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = ramps_transactions))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct TransactionUpdate {
    pub name: Option<String>,
    pub status: Option<Status>,
    pub quote_id: Option<Uuid>,
    pub external_id: Option<String>,
    pub external_fund_id: Option<String>,
    pub transaction_at: Option<DateTime<Utc>>,
    pub metadata: Option<Value>,
    pub from_amount: Option<Element>,
    pub from_currency: Option<Currency>,
    pub from_network: Option<Network>,
    pub to_amount: Option<Element>,
    pub to_currency: Option<Currency>,
    pub to_network: Option<Network>,
    pub evm_address: Option<String>,
    pub private_key: Option<String>,
    pub from_note_kind: Option<Element>,
    pub to_note_kind: Option<Element>,
    pub pending_refund_amount: Option<Element>,
    pub funding_status: Option<FundingStatus>,
    pub funding_due_amount: Option<Element>,
    pub status_reason: Option<TransactionStatusReason>,
    pub memo: Option<String>,
}

impl TransactionUpdate {
    #[must_use]
    pub fn without_metadata(&self) -> Self {
        Self {
            metadata: None,
            ..self.clone()
        }
    }
}
