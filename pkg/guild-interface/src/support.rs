pub use data::support::Status;
use serde::{Deserialize, Serialize};

pub use data::support::SupportIssue;

/// User-visible support status filter exposed by public APIs.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    /// Issues that are open to the user (includes on-hold internally).
    #[default]
    Open,
    /// Issues that have been closed.
    Closed,
}

/// List support issues query
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ListSupportIssuesQuery {
    /// Long poll duration
    pub wait: Option<u64>,
    /// Get changes after unix timestamps (in microseconds)
    pub after: Option<u64>,
    /// Limit result count
    pub limit: Option<u16>,
    /// Filter status of support issue
    pub status: Option<UserStatus>,
}
