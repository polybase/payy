use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use database::schema::ip_data;
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
#[cfg_attr(feature = "diesel", diesel(table_name = ip_data))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct IpData {
    pub id: Uuid,
    pub ip: String,
    pub data: serde_json::Value,
    pub added_at: DateTime<Utc>,
}

#[derive(Clone)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = ip_data))]
pub struct NewIpData<'a> {
    pub ip: &'a str,
    pub data: &'a serde_json::Value,
}
