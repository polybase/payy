use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use database::schema::udh_referral_links;
#[cfg(feature = "diesel")]
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(Queryable, Selectable, Identifiable, AsChangeset)
)]
#[cfg_attr(feature = "diesel", diesel(primary_key(posthog_id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = udh_referral_links))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct UdhReferralLink {
    pub posthog_id: String,
    pub user_device_hash: String,
    pub referrer_url: String,
    pub claimed_at: Option<DateTime<Utc>>,
    pub wallet_id: Option<Uuid>,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = udh_referral_links))]
pub struct NewUdhReferralLink {
    pub posthog_id: String,
    pub user_device_hash: String,
    pub referrer_url: String,
}
