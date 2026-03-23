use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use database::schema::{support_issue_tags, support_tags};
#[cfg(feature = "diesel")]
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "diesel", derive(Queryable, Identifiable))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_tags))]
pub struct SupportTag {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub color: String,
    pub is_active: bool,
    pub added_at: DateTime<Utc>,
    pub auto_close_minutes: Option<i32>,
}

#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_tags))]
pub struct NewSupportTagDb {
    pub name: String,
    pub display_name: String,
    pub color: String,
    pub is_active: bool,
    pub auto_close_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "diesel", derive(Queryable, Identifiable))]
#[cfg_attr(
    feature = "diesel",
    diesel(table_name = support_issue_tags, primary_key(support_issue_id, support_tag_id))
)]
pub struct SupportIssueTag {
    pub support_issue_id: Uuid,
    pub support_tag_id: Uuid,
    pub added_at: DateTime<Utc>,
}

#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_issue_tags))]
pub struct NewSupportIssueTagDb {
    pub support_issue_id: Uuid,
    pub support_tag_id: Uuid,
}
