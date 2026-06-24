use contextful::{FromContextful, InternalError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error, FromContextful)]
pub enum Error {
    #[error("[beam-cli/profiles] profile name cannot be empty")]
    ProfileNameBlank,

    #[error("[beam-cli/profiles] profile already exists: {profile}")]
    ProfileAlreadyExists { profile: String },

    #[error("[beam-cli/profiles] profile not found: {profile}")]
    ProfileNotFound { profile: String },

    #[error("[beam-cli/profiles] grant not found: {grant_id}")]
    GrantNotFound { grant_id: String },

    #[error("[beam-cli/profiles] profiles require a non-empty wallet password")]
    EmptyPasswordWallet,

    #[error("[beam-cli/profiles] profile integrity check failed: {profile}")]
    ProfileIntegrityFailed { profile: String },

    #[error("[beam-cli/profiles] ledger integrity check failed")]
    LedgerIntegrityFailed,

    #[error("[beam-cli/profiles] profile session not found: {profile}")]
    SessionNotFound { profile: String },

    #[error("[beam-cli/profiles] profile session expired: {profile}")]
    SessionExpired { profile: String },

    #[error("[beam-cli/profiles] invalid profile session wallet address for {profile}: {address}")]
    InvalidSessionWalletAddress { profile: String, address: String },

    #[error("[beam-cli/profiles] profile daemon connection failed: {profile}")]
    DaemonConnectionFailed { profile: String },

    #[error("[beam-cli/profiles] profile daemon rejected session token")]
    SessionTokenRejected,

    #[error("[beam-cli/profiles] profile policy denied signing request: {reason}")]
    PolicyDenied { reason: String },

    #[error("[beam-cli/profiles] privacy profile capability is not supported in v1: {capability}")]
    PrivacyCapabilityUnsupported { capability: String },

    #[error("[beam-cli/profiles] invalid profile duration: {value}")]
    InvalidDuration { value: String },

    #[error("[beam-cli/profiles] invalid profile amount: {value}")]
    InvalidAmount { value: String },

    #[error("[beam-cli/profiles] invalid profile daemon request")]
    InvalidDaemonRequest,

    #[error("[beam-cli/profiles] internal error")]
    Internal(#[from] InternalError),
}
