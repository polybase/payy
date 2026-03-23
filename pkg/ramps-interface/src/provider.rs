use currency::Currency;
#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "diesel")]
use std::io::Write;
use strum_macros::{Display, EnumString};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use crate::error::{Error, Result};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Display,
    EnumString,
)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum Provider {
    Alfred,
    Manteca,
    Rain,
    Sumsub,
    Cybrid,
    Stripe,
}

impl Provider {
    /// Derives a provider for a given currency.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedProviderCurrency`] when no provider supports the currency.
    pub fn from_currency(currency: &Currency) -> Result<Self> {
        match currency {
            Currency::ARS => Ok(Self::Manteca),
            Currency::BRL | Currency::MXN => Ok(Self::Alfred),
            _ => Err(Error::UnsupportedProviderCurrency),
        }
    }
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for Provider {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for Provider {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized provider".into())
    }
}
