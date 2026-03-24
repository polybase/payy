use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

use crate::code::ErrorCode;
use crate::error::{ErrorOutput, HTTPError, TryFromHTTPError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestUserData {
    pub id: u64,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationDetails {
    pub field: String,
    pub message: String,
}

#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize, PartialEq,
)]
pub enum SubError {
    #[permission_denied("sub-permission-denied")]
    #[error("[rpc/tests] sub permission denied")]
    PermissionDenied,

    // Skipping Contextful because no extra context is needed in tests.
    #[delegate]
    #[error("[rpc/tests] delegate sub sub")]
    SubDelegate(#[from] SubSubError),
}

#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize, PartialEq,
)]
pub enum SubSubError {
    #[permission_denied("sub-sub-permission-denied")]
    #[error("[rpc/tests] sub sub permission denied")]
    PermissionIsReallyDenied,
}

#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize, PartialEq,
)]
pub enum TestAppError {
    #[bad_request("generic-error")]
    #[error("[rpc/tests] generic error occurred")]
    GenericError,

    #[not_found("user-not-found")]
    #[error("[rpc/tests] user not found: {0:?}")]
    UserNotFound(TestUserData),

    #[bad_request("validation-failed")]
    #[error("[rpc/tests] validation failed")]
    ValidationFailed(String, u32, bool),

    #[already_exists("duplicate-user")]
    #[error("[rpc/tests] duplicate user with id {id} and email {email}")]
    DuplicateUser {
        /// User ID that already exists
        id: u64,
        /// Email address associated with the duplicate user
        email: String,
    },

    #[failed_precondition("complex-error")]
    #[error("[rpc/tests] complex error")]
    ComplexError {
        /// Error code for the complex error
        code: i32,
        /// Whether the system is active
        active: bool,
        /// Optional additional details about the error
        details: Option<String>,
        /// Optional validation metadata
        metadata: Option<ValidationDetails>,
    },

    #[bad_request("warn-severity", severity = "warn")]
    #[error("[rpc/tests] warn severity error")]
    WarnSeverity,

    // Skipping Contextful because no extra context is needed in tests.
    #[delegate]
    #[error("[rpc/tests] delegated error: {0:?}")]
    Delegated(#[from] SubError),
}
