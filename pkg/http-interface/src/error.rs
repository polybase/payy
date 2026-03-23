// lint-long-file-override allow-max-lines=250
use crate::response::HttpMetadata;
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, TryFromHTTPError},
};
use std::{fmt::Debug, marker::PhantomData};

/// Result alias for HTTP client interactions.
pub type Result<T, E> = std::result::Result<T, Error<E>>;

/// Error response from HTTP clients compatible with the workspace APIs.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<E>
where
    E: Debug,
{
    /// RPC error propagated from the destination service.
    #[error("rpc response {0:?}")]
    Rpc(E),
    /// Internal server error.
    #[error("server error {0} at {1}")]
    ServerError(String, HttpMetadata),
    /// Unauthenticated error requiring credential refresh.
    #[error("[http-interface/error] unauthenticated ({reason}) at {metadata}")]
    Unauthenticated {
        /// RPC error payload returned by the destination service.
        output: ErrorOutput,
        /// HTTP request metadata for observability and logging.
        metadata: HttpMetadata,
        /// Reason string explaining why authentication failed.
        reason: String,
    },
    /// Response error format is invalid.
    #[error("unknown response error output {0:?} {1:?} at {2}")]
    UnknownErrorOutput(ErrorOutput, TryFromHTTPError, HttpMetadata),
    /// Error in expected format, but reason code is unknown. Reason code provided.
    #[error("unknown response {0} at {1}")]
    UnknownErrorResponse(String, HttpMetadata),
    /// Serde json parse error.
    #[error("serde_json error {0} at {1}")]
    SerdeJson(String, HttpMetadata),
    /// Reqwest client error.
    #[error("reqwest error {0} at {1}")]
    Reqwest(String, HttpMetadata),
    /// Authentication retrieval failure.
    #[error("unable to get auth {0} at {1}")]
    GetAuth(String, HttpMetadata),
    /// Authentication refresh failure.
    #[error("unable to refresh auth {0} at {1}")]
    RefreshAuth(String, HttpMetadata),
}

/// No RPC errors exist. Its not possible for this error to be constructed.
#[derive(Debug, Clone)]
pub struct NoRpcError(PhantomData<bool>);

/// Canonical authentication error used by [`crate::client::ClientHttpAuth`] implementations.
pub type AuthError = Error<NoRpcError>;

impl TryFrom<ErrorOutput> for NoRpcError {
    type Error = TryFromHTTPError;

    fn try_from(output: ErrorOutput) -> std::result::Result<Self, Self::Error> {
        Err(TryFromHTTPError::NoRpcErrorExpected(output))
    }
}

/// Type-erased error produced during HTTP request execution prior to RPC error conversion.
#[derive(Debug, Clone, thiserror::Error)]
pub enum HttpRequestExecError {
    /// RPC error payload returned by the remote service; conversion deferred until later.
    #[error("[http-interface/error] rpc response {output:?} at {metadata}")]
    Rpc {
        /// Raw error payload from the server.
        output: ErrorOutput,
        /// Request metadata for observability.
        metadata: HttpMetadata,
    },
    /// Internal server error.
    #[error("[http-interface/error] server error {0} at {1}")]
    ServerError(String, HttpMetadata),
    /// Unauthenticated error requiring credential refresh.
    #[error("[http-interface/error] unauthenticated ({reason}) at {metadata}")]
    Unauthenticated {
        /// RPC error payload returned by the destination service.
        output: ErrorOutput,
        /// HTTP request metadata for observability and logging.
        metadata: HttpMetadata,
        /// Reason string explaining why authentication failed.
        reason: String,
    },
    /// Response error format is invalid.
    #[error("[http-interface/error] unknown response {0} at {1}")]
    UnknownErrorResponse(String, HttpMetadata),
    /// Serde json parse error.
    #[error("[http-interface/error] serde_json error {0} at {1}")]
    SerdeJson(String, HttpMetadata),
    /// Reqwest client error.
    #[error("[http-interface/error] reqwest error {message} at {metadata}")]
    Reqwest {
        /// Error message.
        message: String,
        /// Request metadata for observability.
        metadata: HttpMetadata,
        /// Whether the failure was a connection error.
        is_connect: bool,
        /// Whether the failure was a timeout.
        is_timeout: bool,
    },
    /// Authentication retrieval failure.
    #[error("[http-interface/error] unable to get auth {0} at {1}")]
    GetAuth(String, HttpMetadata),
    /// Authentication refresh failure.
    #[error("[http-interface/error] unable to refresh auth {0} at {1}")]
    RefreshAuth(String, HttpMetadata),
}

impl HttpRequestExecError {
    /// Convert the execution error into the public [`Error`] type by materialising the RPC error.
    #[must_use]
    pub fn into_error<E>(self) -> Error<E>
    where
        E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug,
    {
        match self {
            Self::Rpc { output, metadata } => match output.clone().try_into() {
                Ok(rpc_error) => Error::Rpc(rpc_error),
                Err(into_err) => Error::UnknownErrorOutput(output, into_err, metadata),
            },
            Self::ServerError(message, metadata) => Error::ServerError(message, metadata),
            Self::Unauthenticated {
                output,
                metadata,
                reason,
            } => Error::Unauthenticated {
                output,
                metadata,
                reason,
            },
            Self::UnknownErrorResponse(message, metadata) => {
                Error::UnknownErrorResponse(message, metadata)
            }
            Self::SerdeJson(message, metadata) => Error::SerdeJson(message, metadata),
            Self::Reqwest {
                message, metadata, ..
            } => Error::Reqwest(message, metadata),
            Self::GetAuth(message, metadata) => Error::GetAuth(message, metadata),
            Self::RefreshAuth(message, metadata) => Error::RefreshAuth(message, metadata),
        }
    }
}

/// Map the error response from the server into an execution error without materialising RPC types.
pub async fn handle_error_raw(
    response: reqwest::Response,
    method: &reqwest::Method,
    path: &str,
) -> std::result::Result<reqwest::Response, HttpRequestExecError> {
    let status = response.status();
    if status.is_success() || status.is_redirection() {
        return Ok(response);
    }

    let http_metadata = HttpMetadata {
        method: method.clone(),
        path: path.to_owned(),
    };

    let text = response
        .text()
        .await
        .map_err(|err| HttpRequestExecError::Reqwest {
            message: err.to_string(),
            metadata: http_metadata.clone(),
            // Connect/timeout failures happen during request send, not response read.
            is_connect: false,
            is_timeout: false,
        })?;

    let error_output = match serde_json::from_str::<ErrorOutput>(&text) {
        Ok(error) => error,
        Err(err) => {
            if status.is_server_error() {
                return Err(HttpRequestExecError::ServerError(
                    "server error".to_string(),
                    http_metadata,
                ));
            }
            return Err(HttpRequestExecError::UnknownErrorResponse(
                err.to_string(),
                http_metadata,
            ));
        }
    };

    // Check for internal error
    if error_output.error.reason == "internal" || status.is_server_error() {
        return Err(HttpRequestExecError::ServerError(text, http_metadata));
    }

    // Check for invalid authentication, if so we should reauthenticate
    if error_output.error.code == ErrorCode::Unauthenticated {
        return Err(HttpRequestExecError::Unauthenticated {
            reason: error_output.error.reason.clone(),
            output: error_output,
            metadata: http_metadata,
        });
    }

    Err(HttpRequestExecError::Rpc {
        output: error_output,
        metadata: http_metadata,
    })
}

/// Map the error response from the server into the public [`Error`] type.
pub async fn handle_error<E>(
    response: reqwest::Response,
    method: &reqwest::Method,
    path: &str,
) -> Result<reqwest::Response, E>
where
    E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug,
{
    handle_error_raw(response, method, path)
        .await
        .map_err(HttpRequestExecError::into_error::<E>)
}
