use chrono::DateTime;
use chrono::Utc;
#[cfg(feature = "diesel")]
use database::schema::ramps_events;
#[cfg(feature = "diesel")]
use diesel::prelude::*;
#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "diesel")]
use std::io::Write;
use strum_macros::{Display, EnumString};
use uuid::Uuid;

use crate::provider::Provider;

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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum EventSource {
    Webhook,
    Request,
    Response,
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for EventSource {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for EventSource {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse()
            .map_err(|_| "Unrecognized event source kind".into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(Queryable, Insertable, Selectable, Identifiable)
)]
#[cfg_attr(feature = "diesel", diesel(table_name = ramps_events))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Event {
    pub id: Uuid,
    pub provider: Provider,
    pub data: Value,
    pub source: EventSource,
    pub account_id: Option<String>,
    pub transaction_id: Option<String>,
    pub path: Option<String>,
    pub added_at: DateTime<Utc>,
    pub success: Option<bool>,
}
