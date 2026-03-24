use std::num::ParseIntError;

use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::header::ToStrError,
    response::{IntoResponse, Response},
};
use barretenberg_api_interface::ServerError;
use barretenberg_interface::Error as BackendError;
use contextful::Contextful;
use thiserror::Error;
use tokio::task::JoinError;
use tracing::{debug, error, warn};

use crate::server::PermitAcquireError;

#[derive(Debug, Error)]
pub(crate) enum HandlerError {
    #[error("[barretenberg-api-server] permit acquisition error")]
    // Skipping Contextful because no extra context is needed.
    Permit(#[from] PermitAcquireError),
    #[error("[barretenberg-api-server] backend error")]
    Backend(#[from] Contextful<BackendError>),
    #[error("[barretenberg-api-server] backend task failed")]
    TaskJoin(#[from] Contextful<JoinError>),
    #[error("[barretenberg-api-server] invalid json body")]
    InvalidJsonBody(#[from] Contextful<JsonRejection>),
    #[error("[barretenberg-api-server] invalid permit timeout header")]
    InvalidPermitTimeoutHeader(#[from] Contextful<ToStrError>),
    #[error("[barretenberg-api-server] invalid permit timeout value")]
    InvalidPermitTimeoutValue(#[from] Contextful<ParseIntError>),
}

impl From<&HandlerError> for ServerError {
    fn from(err: &HandlerError) -> Self {
        match err {
            HandlerError::Permit(err) => match err {
                PermitAcquireError::Closed => ServerError::ServiceUnavailable {
                    message: "server busy".to_owned(),
                },
                PermitAcquireError::Timeout => ServerError::ServiceUnavailable {
                    message: "timed out waiting for permit".to_owned(),
                },
            },
            HandlerError::Backend(err) => ServerError::Backend(err.source_ref().into()),
            HandlerError::TaskJoin(_) => ServerError::Internal {
                message: "backend task failed".to_owned(),
            },
            HandlerError::InvalidJsonBody(_) => ServerError::InvalidRequest {
                message: "invalid json body".to_owned(),
            },
            HandlerError::InvalidPermitTimeoutHeader(_) => ServerError::InvalidRequest {
                message: "invalid permit timeout header".to_owned(),
            },
            HandlerError::InvalidPermitTimeoutValue(_) => ServerError::InvalidRequest {
                message: "invalid permit timeout value".to_owned(),
            },
        }
    }
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        let server_error = ServerError::from(&self);
        let status = server_error.status_code();

        if matches!(&self, HandlerError::Permit(PermitAcquireError::Timeout)) {
            debug!(
                target: "barretenberg_api_server",
                error = ?self,
                status = status.as_u16(),
                "permit timeout"
            );
        } else if status.is_server_error() {
            error!(
                target: "barretenberg_api_server",
                error = ?self,
                status = status.as_u16(),
                "request failed"
            );
        } else {
            warn!(
                target: "barretenberg_api_server",
                error = ?self,
                status = status.as_u16(),
                "request failed"
            );
        }

        (status, Json(server_error)).into_response()
    }
}
