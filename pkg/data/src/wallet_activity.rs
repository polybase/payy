use chrono::{DateTime, Utc};
#[cfg(feature = "diesel")]
use diesel::{deserialize::FromSqlRow, expression::AsExpression, prelude::*};
use element::Element;
use primitives::serde::{deserialize_base64, serialize_base64};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use uuid::Uuid;

#[cfg(feature = "diesel")]
use crate::derive_pg_text_enum;
#[cfg(feature = "diesel")]
use database::schema::wallet_activity;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset)
)]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = wallet_activity))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct WalletActivity {
    pub id: Uuid,
    pub address: Element,
    pub kind: Kind,
    #[serde(serialize_with = "serialize_base64")]
    #[serde(deserialize_with = "deserialize_base64")]
    pub data: Vec<u8>,
    pub active: bool,
    pub added_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, EnumString, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
pub enum Kind {
    KycV1,
    SendLinkV1,
    ClaimLinkV1,
    SendRegistryV1,
    MintV1,
    CardV1,
    BurnV1,
    RampDepositV1,
    RampDepositLinkV1,
    RampWithdrawV1,
    SupportV1,
    WalletV0,
    MigrateV0,
    AcrossDeposit,
    BungeeDepositV1,
}

#[cfg(feature = "diesel")]
derive_pg_text_enum!(Kind, "SCREAMING_SNAKE_CASE");
