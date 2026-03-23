// lint-long-file-override allow-max-lines=300
#[cfg(feature = "diesel")]
use std::io::Write;

use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::{Jsonb, Text},
};
use serde::{Deserialize, Deserializer, Serialize};
use veil::Redact;

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

pub mod error;

pub use error::{Error, Result};

#[derive(
    Debug, Serialize, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Display, EnumString,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE", ascii_case_insensitive)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature="diesel",diesel(sql_type = diesel::sql_types::Text))]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum Network {
    Payy,
    Polygon,
    Ethereum,
    Spei,
    Pix,
    Coelsa,
    Card,
    ExternalCard,
    UsBank,
    Plaid,
}

impl<'de> Deserialize<'de> for Network {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Try case-insensitive parsing using strum
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for Network {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for Network {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized network".into())
    }
}

/// Provides normalized options to identify an account
/// on a network. A network may support multiple ways
/// to identify an account, and each way may require multiple
/// fields.
#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Redact)]
#[cfg_attr(feature = "diesel", derive(AsExpression, deserialize::FromSqlRow))]
#[cfg_attr(feature="diesel",diesel(sql_type = diesel::sql_types::Jsonb))]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct NetworkIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub accountnumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub routingnumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub cardnumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub cardexpiration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    #[redact]
    pub cardcvv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    #[redact]
    pub pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub phonenumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional, rename = "address"))]
    pub evmaddress: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub methodid: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub accountid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub qrcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub plaid_public_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub plaid_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub plaid_institution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub bank_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub plaid_mask: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub plaid_account_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub external_bank_guid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaidDetails {
    pub plaid_public_token: String,
    pub plaid_account_id: String,
    pub plaid_institution_id: String,
    pub plaid_mask: String,
    pub plaid_account_name: String,
    pub bank_name: String,
}

#[cfg(feature = "diesel")]
impl ToSql<Jsonb, Pg> for NetworkIdentifier {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        out.write_all(&[1])?;
        serde_json::to_writer(out, self)?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Jsonb, Pg> for NetworkIdentifier {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        let bytes = bytes.as_bytes();
        if bytes.is_empty() {
            return Ok(NetworkIdentifier::default());
        }
        let json_bytes = &bytes[1..];
        serde_json::from_slice(json_bytes).map_err(Into::into)
    }
}

impl NetworkIdentifier {
    pub fn from_account(account_number: String) -> Self {
        Self {
            accountnumber: Some(account_number),
            ..Default::default()
        }
    }

    pub fn from_account_and_reference(account_number: String, reference: Option<String>) -> Self {
        Self {
            accountnumber: Some(account_number),
            reference,
            ..Default::default()
        }
    }

    pub fn from_method(method_id: Uuid) -> Self {
        Self {
            methodid: Some(method_id),
            ..Default::default()
        }
    }

    pub fn get_account_number(&self) -> Result<String> {
        ok_field(&self.accountnumber, "accountnumber")
    }

    pub fn get_method_id(&self) -> Result<Uuid> {
        ok_field(&self.methodid, "methodid")
    }

    pub fn get_plaid_details(&self) -> Result<PlaidDetails> {
        Ok(PlaidDetails {
            plaid_public_token: ok_field(&self.plaid_public_token, "plaid_public_token")?,
            plaid_mask: ok_field(&self.plaid_mask, "plaid_mask")?,
            plaid_account_name: ok_field(&self.plaid_account_name, "plaid_account_name")?,
            plaid_institution_id: ok_field(&self.plaid_institution_id, "plaid_institution_id")?,
            plaid_account_id: ok_field(&self.plaid_account_id, "plaid_account_id")?,
            bank_name: ok_field(&self.bank_name, "bank_name")?,
        })
    }
}

pub fn ok_field<T: Clone>(field: &Option<T>, field_name: &'static str) -> Result<T> {
    field
        .clone()
        .ok_or(Error::MissingRequiredNetworkIdentifierField(
            field_name.to_string(),
        ))
}
