use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use database::schema::diagnostics;
#[cfg(feature = "diesel")]
use diesel::{
    Selectable,
    prelude::{Insertable, Queryable},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Insertable, Queryable, Selectable))]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = diagnostics))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Diagnostics {
    pub id: Uuid,
    pub wallet_id: Option<Uuid>,
    pub address: String,
    pub backup_diffs: serde_json::Value,
    pub state: serde_json::Value,
    pub mnemonic: String,
    pub device_info: serde_json::Value,
    pub message: Option<String>,
    pub added_at: DateTime<Utc>,
}

#[derive(Clone)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = diagnostics))]
pub struct NewDiagnostics {
    pub wallet_id: Option<Uuid>,
    pub address: String,
    pub backup_diffs: serde_json::Value,
    pub state: serde_json::Value,
    pub mnemonic: String,
    pub device_info: serde_json::Value,
    pub message: Option<String>,
}
