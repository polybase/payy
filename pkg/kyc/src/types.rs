#[cfg(feature = "diesel")]
use std::io::Write;

#[cfg(feature = "diesel")]
use diesel::{
    AsExpression,
    deserialize::{self, FromSql, FromSqlRow},
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

#[derive(
    Debug, Serialize, Deserialize, Copy, Clone, Eq, Default, PartialEq, Display, EnumString,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature="diesel",diesel(sql_type = diesel::sql_types::Text))]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum KycStatus {
    #[default]
    NotStarted,
    Pending,
    Approved,
    UpdateRequired,
    Rejected,
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for KycStatus {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for KycStatus {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized kyc status".into())
    }
}

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::EnumIter,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum KycField {
    Firstname,
    Middlename,
    Lastname,
    Dob,
    Occupation,
    AddressStreet,
    AddressCity,
    AddressState,
    AddressCountry,
    AddressPostalCode,
    Nationalities,
    Documents,
    Phone,
    PhoneVerified,
    Email,
    EmailVerified,
    NationalId,
    CivilState,
    Pep,
    Fatca,
    Uif,
}
