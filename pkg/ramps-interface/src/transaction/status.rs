#[cfg(feature = "diesel")]
use std::io::Write;

#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql},
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Display,
    EnumString,
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum FundingStatus {
    PendingDebit,
    PendingCredit,
    InsufficientFunds,
    Settled,
}

#[cfg(feature = "diesel")]
impl ToSql<diesel::sql_types::Text, Pg> for FundingStatus {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<diesel::sql_types::Text, Pg> for FundingStatus {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized funding status".into())
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Display,
    EnumString,
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatusReason {
    DepositAmountTooLow,
    DepositAmountTooHigh,
}

#[cfg(feature = "diesel")]
impl ToSql<diesel::sql_types::Text, Pg> for TransactionStatusReason {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<diesel::sql_types::Text, Pg> for TransactionStatusReason {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse()
            .map_err(|_| "Unrecognized Transaction cancel reason".into())
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Display,
    EnumString,
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(rename = "RampsTransactionStatus"))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum Status {
    /// Pending status - default initial status
    Pending,
    /// Transaction is complete, no further changes expected
    Complete,
    /// Cancelled by either the user or the provider
    Cancelled,
    /// Txn was refunded back to the user in full
    Refunded,
    /// Card only
    Declined,
    /// An error caused the request to fail
    Failed,
    // Onramp/offramps only
    /// Transaction has been funded, we have received the users funds
    Funded,
    /// Funds have been withdrawn/sent to the user
    Withdraw,
}

#[cfg(feature = "diesel")]
impl ToSql<diesel::sql_types::Text, Pg> for Status {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<diesel::sql_types::Text, Pg> for Status {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized status".into())
    }
}
