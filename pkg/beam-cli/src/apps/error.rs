use contextful::{FromContextful, InternalError};

use crate::apps::model::PrivacyCapability;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error, FromContextful)]
pub enum Error {
    #[error("[beam-cli/apps] app not installed: {app}")]
    AppNotInstalled { app: String },

    #[error("[beam-cli/apps] unknown app in registry: {app}")]
    RegistryAppNotFound { app: String },

    #[error("[beam-cli/apps] version not found for {app}: {version}")]
    RegistryVersionNotFound { app: String, version: String },

    #[error("[beam-cli/apps] app id mismatch: expected {expected}, got {actual}")]
    AppIdMismatch { actual: String, expected: String },

    #[error("[beam-cli/apps] app version mismatch: expected {expected}, got {actual}")]
    AppVersionMismatch { actual: String, expected: String },

    #[error("[beam-cli/apps] invalid app id: {value}")]
    InvalidAppId { value: String },

    #[error("[beam-cli/apps] invalid app command: {value}")]
    InvalidCommandName { value: String },

    #[error("[beam-cli/apps] invalid permission glob: {value}")]
    InvalidPermissionGlob { value: String },

    #[error("[beam-cli/apps] invalid permission url: {value}")]
    InvalidPermissionUrl { value: String },

    #[error("[beam-cli/apps] invalid registry url: {value}")]
    InvalidRegistryUrl { value: String },

    #[error("[beam-cli/apps] invalid permission selector: {value}")]
    InvalidPermissionSelector { value: String },

    #[error("[beam-cli/apps] invalid digest for {artifact}: {digest}")]
    InvalidDigest { artifact: String, digest: String },

    #[error("[beam-cli/apps] digest mismatch for {artifact}: expected {expected}, got {actual}")]
    DigestMismatch {
        actual: String,
        artifact: String,
        expected: String,
    },

    #[error("[beam-cli/apps] unsupported Beam version for {app}: requires {required}")]
    UnsupportedBeamVersion { app: String, required: String },

    #[error("[beam-cli/apps] registry signature mismatch for {artifact}")]
    SignatureMismatch { artifact: String },

    #[error("[beam-cli/apps] app module is not a wasm module: {app}")]
    InvalidWasmModule { app: String },

    #[error("[beam-cli/apps] app requested blocked contract target: {target}")]
    ContractPermissionDenied { target: String },

    #[error("[beam-cli/apps] app requested blocked chain operation on {chain}: {operation}")]
    ChainPermissionDenied { chain: String, operation: String },

    #[error("[beam-cli/apps] app requested blocked selector: {selector}")]
    SelectorPermissionDenied { selector: String },

    #[error("[beam-cli/apps] app requested blocked approval spender: {spender}")]
    SpenderPermissionDenied { spender: String },

    #[error("[beam-cli/apps] app requested blocked http url: {url}")]
    HttpPermissionDenied { url: String },

    #[error("[beam-cli/apps] app requested unsafe http host: {host}")]
    HttpHostDenied { host: String },

    #[error("[beam-cli/apps] app http request too large: {bytes} bytes")]
    HttpRequestTooLarge { bytes: usize },

    #[error("[beam-cli/apps] app http response too large: {bytes} bytes")]
    HttpResponseTooLarge { bytes: usize },

    #[error("[beam-cli/apps] app http redirect limit exceeded: {url}")]
    HttpRedirectLimitExceeded { url: String },

    #[error("[beam-cli/apps] app host api request is invalid: {reason}")]
    InvalidHostRequest { reason: String },

    #[error("[beam-cli/apps] app action requires approval")]
    ApprovalRequired,

    #[error("[beam-cli/apps] app approval rejected")]
    ApprovalRejected,

    #[error("[beam-cli/apps] approval not found: {approval_id}")]
    ApprovalNotFound { approval_id: String },

    #[error("[beam-cli/apps] approval expired: {approval_id}")]
    ApprovalExpired { approval_id: String },

    #[error("[beam-cli/apps] approval is not pending: {approval_id}")]
    ApprovalNotPending { approval_id: String },

    #[error("[beam-cli/apps] approval app artifact changed: {approval_id}")]
    ApprovalArtifactChanged { approval_id: String },

    #[error("[beam-cli/apps] approval plan changed: {approval_id}")]
    ApprovalPlanChanged { approval_id: String },

    #[error(
        "[beam-cli/apps] approval execution context changed for {approval_id}: {field} expected {expected}, got {actual}"
    )]
    ApprovalContextChanged {
        actual: String,
        approval_id: String,
        expected: String,
        field: String,
    },

    #[error("[beam-cli/apps] unsupported app command: {command}")]
    UnsupportedAppCommand { command: String },

    #[error("[beam-cli/apps] unsupported privacy app capability: {capability:?}")]
    UnsupportedPrivacyCapability { capability: PrivacyCapability },

    #[error("[beam-cli/apps] internal error")]
    Internal(#[from] InternalError),
}
