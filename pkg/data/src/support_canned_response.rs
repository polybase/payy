use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use database::schema::{support_canned_response_tags, support_canned_responses};
#[cfg(feature = "diesel")]
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "diesel", derive(Queryable, Identifiable))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_canned_responses))]
pub struct SupportCannedResponse {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub content: String,
    pub is_active: bool,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_canned_responses))]
pub struct NewSupportCannedResponseDb {
    pub name: String,
    pub display_name: String,
    pub content: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_canned_responses))]
pub struct UpdateSupportCannedResponseDb {
    pub display_name: Option<String>,
    pub content: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "diesel", derive(Queryable, Identifiable))]
#[cfg_attr(
    feature = "diesel",
    diesel(
        table_name = support_canned_response_tags,
        primary_key(support_canned_response_id, support_tag_id)
    )
)]
pub struct SupportCannedResponseTag {
    pub support_canned_response_id: Uuid,
    pub support_tag_id: Uuid,
    pub added_at: DateTime<Utc>,
}

#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = support_canned_response_tags))]
pub struct NewSupportCannedResponseTagDb {
    pub support_canned_response_id: Uuid,
    pub support_tag_id: Uuid,
}
