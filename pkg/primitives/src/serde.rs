use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql},
    pg::Pg,
    serialize::{self, Output, ToSql},
    sql_types::Bytea,
};

/// Wrapper for `Vec<u8>` that (de)serializes to/from base64 strings.
///
/// Using this type instead of raw `Vec<u8>` avoids sprinkling `#[serde(serialize_with = ...)]`
/// attributes throughout the codebase while preserving the JSON contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(transparent)]
#[cfg_attr(feature = "diesel", derive(diesel::AsExpression, diesel::FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Bytea))]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct Base64Bytes(
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    #[cfg_attr(feature = "ts-rs", ts(as = "String"))]
    pub Vec<u8>,
);

impl Base64Bytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Deref for Base64Bytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Base64Bytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<[u8]> for Base64Bytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsMut<[u8]> for Base64Bytes {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl From<Vec<u8>> for Base64Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self::new(bytes)
    }
}

impl From<&[u8]> for Base64Bytes {
    fn from(bytes: &[u8]) -> Self {
        Self::new(bytes.to_vec())
    }
}

impl From<Base64Bytes> for Vec<u8> {
    fn from(bytes: Base64Bytes) -> Self {
        bytes.into_inner()
    }
}

#[cfg(feature = "diesel")]
impl ToSql<Bytea, Pg> for Base64Bytes {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        <Vec<u8> as ToSql<Bytea, Pg>>::to_sql(&self.0, out)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Bytea, Pg> for Base64Bytes {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        <Vec<u8> as FromSql<Bytea, Pg>>::from_sql(bytes).map(Base64Bytes::new)
    }
}

// Custom serializer for Vec<u8> to base64 string
pub fn serialize_base64<S>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let base64_string = base64::engine::general_purpose::STANDARD.encode(value);
    serializer.serialize_str(&base64_string)
}

// Custom deserializer for base64 string to Vec<u8>
pub fn deserialize_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let trimmed = s.trim();
    base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .map_err(serde::de::Error::custom)
}

pub fn serialize_hex_0x_prefixed<S>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex_string = format!("0x{}", hex::encode(value));
    serializer.serialize_str(&hex_string)
}

pub fn deserialize_hex_0x_prefixed<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let s = s.trim_start_matches("0x");
    hex::decode(s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests;
