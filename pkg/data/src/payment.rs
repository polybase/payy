#[cfg(feature = "diesel")]
use database::schema::payments;
#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    prelude::*,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;
#[cfg(feature = "diesel")]
use std::io::Write;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(Insertable, Queryable, Selectable, Identifiable, AsChangeset)
)]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = payments))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Payment {
    pub id: Uuid,
    pub product: String,
    pub provider: String,
    pub external_id: Option<String>,
    pub amount: i32,
    pub currency: PaymentCurrency,
    pub status: PaymentStatus,
    pub payment_by: Option<String>,
}

#[derive(Clone)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = payments))]
pub struct NewPayment<'a> {
    pub product: &'a str,
    pub provider: &'a str,
    pub data: &'a Value,
    pub external_id: &'a str,
    pub amount: &'a i32,
    pub currency: &'a PaymentCurrency,
    pub status: &'a PaymentStatus,
    pub payment_by: Option<&'a str>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
pub enum PaymentStatus {
    // Payment process started, but not sent to provider
    Pending,
    // Payment request has been sent
    Requested,
    // Payment is approved and ready for product to be delivered
    Approved,
    // Payment is cancelled by the user
    Cancelled,
    // Payment failed
    Failed,
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for PaymentStatus {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        match *self {
            PaymentStatus::Pending => out.write_all(b"pending")?,
            PaymentStatus::Requested => out.write_all(b"requested")?,
            PaymentStatus::Approved => out.write_all(b"approved")?,
            PaymentStatus::Cancelled => out.write_all(b"cancelled")?,
            PaymentStatus::Failed => out.write_all(b"failed")?,
        }
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for PaymentStatus {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"pending" => Ok(PaymentStatus::Pending),
            b"requested" => Ok(PaymentStatus::Requested),
            b"approved" => Ok(PaymentStatus::Approved),
            b"cancelled" => Ok(PaymentStatus::Cancelled),
            b"failed" => Ok(PaymentStatus::Failed),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
pub enum PaymentCurrency {
    Usd,
}

#[cfg(feature = "stripe")]
impl From<PaymentCurrency> for stripe::Currency {
    fn from(value: PaymentCurrency) -> Self {
        match value {
            PaymentCurrency::Usd => stripe::Currency::USD,
        }
    }
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for PaymentCurrency {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        match *self {
            PaymentCurrency::Usd => out.write_all(b"USD")?,
        }
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for PaymentCurrency {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"USD" => Ok(PaymentCurrency::Usd),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl Display for PaymentCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentCurrency::Usd => write!(f, "USD"),
        }
    }
}
