#![deny(unsafe_code)]

//! # Barretenberg API Interface
//!
//! This crate defines the shared types and error handling contract for the Barretenberg API.
//!
//! ## Error Handling Philosophy: Gateway Disambiguation
//!
//! A key design choice in this API is the separation of application-level errors from
//! infrastructure-level errors (gateways, proxies, load balancers).
//!
//! To achieve this, **the Barretenberg API server never returns HTTP 5xx status codes.**
//!
//! *   **HTTP 4xx**: Indicates the request reached the application. This includes logical
//!     errors (Invalid Request, Verification Failed) AND internal server failures (Task Panics,
//!     Database failures). By using 4xx (specifically `422 Unprocessable Entity` for internal
//!     errors), we ensure that a 5xx response is an unambiguous signal that the failure
//!     occurred in the surrounding infrastructure, not the application logic.
//! *   **HTTP 5xx**: Indicates a failure in the gateway or network layer.
//!
//! Clients should monitor for `ServerError` payloads in 4xx responses to track application health.

use barretenberg_interface::Error as BackendError;
use primitives::serde::Base64Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PERMIT_TIMEOUT_HEADER: &str = "x-barretenberg-permit-timeout-ms";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProveRequest {
    pub program: Base64Bytes,
    pub bytecode: Base64Bytes,
    pub key: Base64Bytes,
    pub witness: Base64Bytes,
    #[serde(default)]
    pub oracle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProveResponse {
    pub proof: Base64Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub proof: Base64Bytes,
    pub public_inputs: Base64Bytes,
    pub key: Base64Bytes,
    #[serde(default)]
    pub oracle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "code", content = "data", rename_all = "kebab-case")]
pub enum ServerError {
    /// The server is busy or timed out waiting for resources (e.g. permit).
    /// Corresponds to HTTP 429 Too Many Requests.
    #[error("[barretenberg-api] service unavailable: {message}")]
    ServiceUnavailable { message: String },

    /// The request was invalid (bad JSON, missing headers, etc.).
    /// Corresponds to HTTP 400 Bad Request.
    #[error("[barretenberg-api] invalid request: {message}")]
    InvalidRequest { message: String },

    /// An error occurred in the underlying Barretenberg backend.
    /// Corresponds to HTTP 422 Unprocessable Entity (VerificationFailed) or 400 Bad Request (Backend).
    #[error("[barretenberg-api] backend error")]
    Backend(#[from] BbBackendError),

    /// An internal server error occurred (not related to backend logic).
    ///
    /// Note: We use HTTP 422 Unprocessable Entity to distinguish application-level
    /// crashes from infrastructure-level (gateway) 5xx errors.
    #[error("[barretenberg-api] internal error: {message}")]
    Internal { message: String },
}

impl ServerError {
    pub fn status_code(&self) -> http::StatusCode {
        match self {
            ServerError::ServiceUnavailable { .. } => http::StatusCode::TOO_MANY_REQUESTS,
            ServerError::InvalidRequest { .. } => http::StatusCode::BAD_REQUEST,
            ServerError::Backend(err) => match err {
                BbBackendError::VerificationFailed => http::StatusCode::UNPROCESSABLE_ENTITY,
                BbBackendError::Backend { .. } => http::StatusCode::BAD_REQUEST,
                BbBackendError::ImplementationSpecific { .. } => {
                    http::StatusCode::UNPROCESSABLE_ENTITY
                }
            },
            ServerError::Internal { .. } => http::StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

/// A serializable error type representing failures from the Barretenberg backend.
///
/// This type mirrors `barretenberg_interface::Error` but is designed to be stable
/// and serializable for API communication. It decouples the API contract from
/// the internal backend error implementation, allowing the backend to evolve
/// without breaking API clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(rename_all = "kebab-case")]
pub enum BbBackendError {
    /// Corresponds to barretenberg_interface::Error::Backend(String)
    #[error("[barretenberg-backend] backend: {message}")]
    Backend { message: String },
    /// Corresponds to barretenberg_interface::Error::VerificationFailed
    #[error("[barretenberg-backend] verification failed")]
    VerificationFailed,
    /// Corresponds to `barretenberg_interface::Error::ImplementationSpecific(Box<dyn std::error::Error>)`
    /// We stringify the error since we can't serialize `Box<dyn std::error::Error>`.
    #[error("[barretenberg-backend] implementation specific: {message}")]
    ImplementationSpecific { message: String },
}

impl From<BackendError> for BbBackendError {
    fn from(err: BackendError) -> Self {
        match err {
            BackendError::Backend(message) => BbBackendError::Backend { message },
            BackendError::VerificationFailed => BbBackendError::VerificationFailed,
            BackendError::ImplementationSpecific(err) => BbBackendError::ImplementationSpecific {
                message: err.to_string(),
            },
        }
    }
}

impl From<&BackendError> for BbBackendError {
    fn from(err: &BackendError) -> Self {
        match err {
            BackendError::Backend(message) => BbBackendError::Backend {
                message: message.clone(),
            },
            BackendError::VerificationFailed => BbBackendError::VerificationFailed,
            BackendError::ImplementationSpecific(err) => BbBackendError::ImplementationSpecific {
                message: err.to_string(),
            },
        }
    }
}

impl From<BbBackendError> for BackendError {
    fn from(err: BbBackendError) -> Self {
        match err {
            BbBackendError::Backend { message } => BackendError::Backend(message),
            BbBackendError::VerificationFailed => BackendError::VerificationFailed,
            BbBackendError::ImplementationSpecific { message } => {
                BackendError::ImplementationSpecific(Box::new(ImplementationSpecificError {
                    message,
                }))
            }
        }
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
struct ImplementationSpecificError {
    message: String,
}

impl From<BackendError> for ServerError {
    fn from(err: BackendError) -> Self {
        ServerError::Backend(BbBackendError::from(err))
    }
}
