use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use database::schema::registry_notes;
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
#[cfg_attr(feature = "diesel", diesel(table_name = registry_notes))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct RegistryNote {
    pub id: Uuid,
    pub block: i64,
    pub public_key: String,
    pub encrypted_key: Vec<u8>,
    pub encrypted_note: Vec<u8>,
    pub added_at: DateTime<Utc>,
}
