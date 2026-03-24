use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use database::schema::wallets;
#[cfg(feature = "diesel")]
use diesel::{
    Selectable,
    prelude::{AsChangeset, Queryable},
};
use kyc::Kyc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Queryable, Selectable))]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallets))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Wallet {
    pub id: Uuid,
    pub address: String,
    pub expo_push_token: Option<String>,
    pub deposit_address: Option<String>,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub atlas_customer_id: Option<String>,
    pub kyc: Option<Kyc>,
    pub country: Option<String>,
    pub language: Option<String>,
    pub ip_country: Option<String>,
    pub data: Option<serde_json::Value>,
    #[cfg_attr(feature = "diesel", diesel(deserialize_as = i16))]
    pub version: u16,
    pub fraud_block: bool,
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallets))]
pub struct UpdateWallet {
    pub address: Option<String>,
    pub expo_push_token: Option<String>,
    pub deposit_address: Option<String>,
    pub atlas_customer_id: Option<String>,
    pub kyc: Option<Kyc>,
    pub country: Option<String>,
    pub language: Option<String>,
    pub ip_country: Option<String>,
    pub version: Option<i16>,
    pub fraud_block: Option<bool>,
}
