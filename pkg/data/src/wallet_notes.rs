use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use diesel::{deserialize::FromSqlRow, expression::AsExpression, prelude::*};
use element::Element;
use primitives::serde::Base64Bytes;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use uuid::Uuid;

#[cfg(feature = "diesel")]
use crate::derive_pg_text_enum;
#[cfg(feature = "diesel")]
use database::schema::wallet_notes;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset)
)]
#[cfg_attr(feature = "diesel", diesel(primary_key(commitment)))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_notes))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct WalletNote {
    pub commitment: Element,
    pub address: Element,
    pub data: Base64Bytes,
    pub status: Status,
    pub activity_id: Option<Uuid>,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, EnumString, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
pub enum Status {
    Unspent,
    Spent,
    NotFound,
    Dropped,
}

#[cfg(feature = "diesel")]
derive_pg_text_enum!(Status, "SCREAMING_SNAKE_CASE");
