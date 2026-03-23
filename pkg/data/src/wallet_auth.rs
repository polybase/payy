use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use database::schema::wallet_auths;
#[cfg(feature = "diesel")]
use diesel::{
    Selectable,
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    prelude::{AsChangeset, Insertable, Queryable},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "diesel")]
use std::io::Write;
use std::{fmt, str::FromStr};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "diesel", diesel(sql_type = Text))]
pub enum WalletAuthKind {
    ApiKey,
}

impl fmt::Display for WalletAuthKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalletAuthKind::ApiKey => write!(f, "API_KEY"),
        }
    }
}

impl FromStr for WalletAuthKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "API_KEY" => Ok(WalletAuthKind::ApiKey),
            _ => Err(format!("Unknown WalletAuthKind: {s}")),
        }
    }
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for WalletAuthKind {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        out.write_all(self.to_string().as_bytes())?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for WalletAuthKind {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|e: String| e.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyValue {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum WalletAuthValue {
    ApiKey(ApiKeyValue),
}

impl WalletAuthValue {
    pub fn kind(&self) -> WalletAuthKind {
        match self {
            WalletAuthValue::ApiKey(_) => WalletAuthKind::ApiKey,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Queryable, Selectable))]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_auths))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct WalletAuth {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub kind: WalletAuthKind,
    pub value: String,
    pub enabled: bool,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_auths))]
pub struct NewWalletAuth {
    pub wallet_id: Uuid,
    pub kind: WalletAuthKind,
    pub value: String,
    pub enabled: bool,
}

impl NewWalletAuth {
    pub fn new_api_key(wallet_id: Uuid, api_key: String, enabled: bool) -> Self {
        Self {
            wallet_id,
            kind: WalletAuthKind::ApiKey,
            value: api_key,
            enabled,
        }
    }
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_auths))]
pub struct UpdateWalletAuth {
    pub kind: Option<WalletAuthKind>,
    pub value: Option<String>,
    pub enabled: Option<bool>,
    pub updated_at: Option<DateTime<Utc>>,
}
