use crate::util::{deserialize_datetime_opt, serialize_datetime_opt};
use chrono::{DateTime, Utc};
pub use data::wallet_activity::{Kind, WalletActivity};
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// Data for address mismatch error
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AddressMismatch {
    /// Address of authenticated user
    pub authenticated_address: String,
    /// Address in the activity body
    pub activity_address: String,
}

/// RPC errors for activity
#[derive(Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize)]
pub enum Error {
    /// Invalid signature length, epected uncompressed signature
    #[bad_request("wallet-activity-address-mismatch")]
    #[error("authenticated address does not match activity address")]
    AuthAddressMismatch(AddressMismatch),
    /// Activity item not found
    #[error("wallet activity not found")]
    #[not_found("activity-not-found")]
    ActivityNotFound,
    /// Activity conflict due to stale client data
    #[failed_precondition("activity-stale-data")]
    #[error("[wallet_activity] activity data has been modified since last read")]
    ActivityConflict {
        /// Latest server activity data to help clients reconcile
        latest: WalletActivity,
    },
}

/// Query for a list of wallet activity records
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WalletActivityListQuery {
    /// List activity after this point
    #[serde(
        default,
        serialize_with = "serialize_datetime_opt",
        deserialize_with = "deserialize_datetime_opt"
    )]
    pub after: Option<DateTime<Utc>>,
    /// List activity before this point
    #[serde(
        default,
        serialize_with = "serialize_datetime_opt",
        deserialize_with = "deserialize_datetime_opt"
    )]
    pub before: Option<DateTime<Utc>>,
    /// Number of records to return
    pub limit: Option<u64>,
    /// Filter for active records
    pub active: Option<bool>,
}

impl WalletActivityListQuery {
    #[must_use]
    /// Set limit query
    pub fn limit(self, limit: u64) -> Self {
        Self {
            limit: Some(limit),
            ..self
        }
    }

    #[must_use]
    /// Set after query
    pub fn after(self, after: DateTime<Utc>) -> Self {
        Self {
            after: Some(after),
            ..self
        }
    }

    #[must_use]
    /// Set before query
    pub fn before(self, before: DateTime<Utc>) -> Self {
        Self {
            before: Some(before),
            ..self
        }
    }

    #[must_use]
    /// Set after query
    pub fn active(self) -> Self {
        Self {
            active: Some(true),
            ..self
        }
    }
}

/// Upsert payload for wallet activity requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletActivityUpsert {
    /// Activity payload to create or update
    #[serde(flatten)]
    pub activity: WalletActivity,
    /// Timestamp of the client's last synchronized update
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_datetime_opt",
        deserialize_with = "deserialize_datetime_opt"
    )]
    pub last_updated_at: Option<DateTime<Utc>>,
}
