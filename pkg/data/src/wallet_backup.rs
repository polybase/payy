use chrono::{Utc, serde::ts_milliseconds};
#[cfg(feature = "diesel")]
use database::schema::{wallet_backup_tags, wallet_backups};
#[cfg(feature = "diesel")]
use diesel::prelude::*;
use primitives::serde::Base64Bytes;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Queryable, Selectable, AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_backups))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct WalletBackup {
    pub wallet_address: String,
    pub last_update: String,
    pub backup_path: String,
    pub backup_hash: Vec<u8>,
    #[serde(with = "ts_milliseconds")]
    pub added_at: chrono::DateTime<Utc>,
    pub diff_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBackupWithData {
    #[serde(flatten)]
    pub wallet_meta: WalletBackup,
    pub backup: Base64Bytes,
    pub backup_diff: Option<Base64Bytes>,
}

#[derive(Clone)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_backups))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct NewWalletBackup<'a> {
    pub wallet_address: &'a str,
    pub last_update: &'a str,
    pub backup_path: &'a str,
    pub backup_hash: &'a [u8],
    pub diff_of: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "diesel", derive(Queryable, Selectable, AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_backup_tags))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct WalletBackupTag {
    pub wallet_address: String,
    /// This is always "latest" for now
    pub tag: String,
    pub last_update: String,
}

#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_backup_tags))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct NewWalletBackupTag {
    pub wallet_address: String,
    /// This is always "latest" for now
    pub tag: String,
    pub last_update: String,
}
