// lint-long-file-override allow-max-lines=300
use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use diesel::{deserialize::FromSqlRow, expression::AsExpression, prelude::*};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use uuid::Uuid;

#[cfg(feature = "diesel")]
use crate::derive_pg_text_enum;
#[cfg(feature = "diesel")]
use database::schema::{support_issues, support_messages};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Queryable))]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct SupportIssue {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub external_id: Uuid,
    pub status: Status,
    pub channel: String,
    pub subject: Option<String>,
    pub unread_count: i32,
    pub last_message: String,
    pub last_message_at: DateTime<Utc>,
    pub last_read_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub added_at: DateTime<Utc>,
    #[cfg_attr(feature = "ts-rs", ts(type = "unknown | null"))]
    pub metadata: Option<serde_json::Value>,
    pub auto_close_at: Option<DateTime<Utc>>,
    pub auto_close_minutes: Option<i32>,
    pub last_ai_support_bot_processed_at: Option<DateTime<Utc>>,
}

#[derive(
    Debug, Default, EnumString, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum Status {
    #[default]
    Open,
    Closed,
    OnHold,
}

#[cfg(feature = "diesel")]
derive_pg_text_enum!(Status, "SCREAMING_SNAKE_CASE");

#[derive(
    Debug, Default, EnumString, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum Role {
    #[default]
    User,
    Agent,
}

#[cfg(feature = "diesel")]
derive_pg_text_enum!(Role, "SCREAMING_SNAKE_CASE");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
#[cfg_attr(feature = "diesel", derive(Queryable))]
pub struct SupportMessage {
    pub id: Uuid,
    pub support_issue_id: Uuid,
    pub external_id: String,
    pub emoji: Option<String>,
    pub role: Role,
    pub message: String,
    #[cfg_attr(feature = "ts-rs", ts(type = "{}"))]
    pub attachment: Option<serde_json::Value>,
    pub added_at: DateTime<Utc>,
    pub is_bot: bool,
    pub is_internal: bool,
    pub agent_id: Option<i32>,
}

#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_messages))]
pub struct NewSupportMessageDb {
    pub id: Uuid,
    pub support_issue_id: Uuid,
    pub external_id: String,
    pub role: Role,
    pub message: String,
    pub emoji: Option<String>,
    pub attachment: Option<serde_json::Value>,
    pub added_at: DateTime<Utc>,
    pub is_bot: bool,
    pub is_internal: bool,
    pub agent_id: Option<i32>,
}

#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_issues))]
pub struct NewSupportIssueDb {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub external_id: Uuid,
    pub status: String,
    pub channel: String,
    pub subject: Option<String>,
    pub unread_count: i32,
    pub last_message: String,
    pub last_message_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_issues))]
#[derive(Default, Debug)]
pub struct UpdateSupportIssueDb {
    pub status: Option<String>,
    pub subject: Option<String>,
    pub unread_count: Option<i32>,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub last_read_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub auto_close_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub auto_close_minutes: Option<i32>,
    pub last_ai_support_bot_processed_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use serde_json;

    #[test]
    fn test_status_serialization() {
        assert_eq!(serde_json::to_string(&Status::Open).unwrap(), "\"OPEN\"");
        assert_eq!(
            serde_json::to_string(&Status::Closed).unwrap(),
            "\"CLOSED\""
        );
        assert_eq!(
            serde_json::to_string(&Status::OnHold).unwrap(),
            "\"ON_HOLD\""
        );
    }

    #[test]
    fn test_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<Status>("\"OPEN\"").unwrap(),
            Status::Open
        );
        assert_eq!(
            serde_json::from_str::<Status>("\"CLOSED\"").unwrap(),
            Status::Closed
        );
        assert_eq!(
            serde_json::from_str::<Status>("\"ON_HOLD\"").unwrap(),
            Status::OnHold
        );
    }

    #[test]
    fn test_status_display() {
        assert_eq!(Status::Open.to_string(), "OPEN");
        assert_eq!(Status::Closed.to_string(), "CLOSED");
        assert_eq!(Status::OnHold.to_string(), "ON_HOLD");
    }

    #[test]
    fn test_status_from_string() {
        assert_eq!(Status::from_str("OPEN").unwrap(), Status::Open);
        assert_eq!(Status::from_str("CLOSED").unwrap(), Status::Closed);
        assert_eq!(Status::from_str("ON_HOLD").unwrap(), Status::OnHold);
    }

    #[test]
    fn test_status_default() {
        assert_eq!(Status::default(), Status::Open);
    }

    #[test]
    fn test_role_serialization() {
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"USER\"");
        assert_eq!(serde_json::to_string(&Role::Agent).unwrap(), "\"AGENT\"");
    }

    #[test]
    fn test_role_deserialization() {
        assert_eq!(
            serde_json::from_str::<Role>("\"USER\"").unwrap(),
            Role::User
        );
        assert_eq!(
            serde_json::from_str::<Role>("\"AGENT\"").unwrap(),
            Role::Agent
        );
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::User.to_string(), "USER");
        assert_eq!(Role::Agent.to_string(), "AGENT");
    }

    #[test]
    fn test_role_from_string() {
        assert_eq!(Role::from_str("USER").unwrap(), Role::User);
        assert_eq!(Role::from_str("AGENT").unwrap(), Role::Agent);
    }

    #[test]
    fn test_role_default() {
        assert_eq!(Role::default(), Role::User);
    }
}
