// lint-long-file-override allow-max-lines=300
use std::fmt::Debug;
use std::{error::Error, fmt::Display};

use actix_web::{HttpResponse, Responder, ResponseError, http::header::ContentType};
use contextful::Contextful;
use serde::{Deserialize, Serialize};

use crate::code::ErrorCode;

pub type HttpResult<T> = std::result::Result<T, HTTPError>;

/// Variant `Error` is the default error level.
/// `Warn` is to be used for "expected" errors that we wish
/// to avoid polluting the error logs.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Warn,
    #[default]
    Error,
}

#[derive(Debug)]
pub struct HTTPError {
    pub code: ErrorCode,
    pub reason: String,
    pub source: Option<Box<dyn Error>>,
    pub data: Option<serde_json::Value>,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOutput {
    pub error: ErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: ErrorCode,
    pub reason: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// Define a custom error for TryFrom conversion failures
#[derive(Debug, Clone)]
pub enum TryFromHTTPError {
    NoRpcErrorExpected(ErrorOutput),
    UnknownReason(String),
    DeserializationError,
    MissingData,
}

impl std::fmt::Display for TryFromHTTPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryFromHTTPError::NoRpcErrorExpected(output) => {
                write!(f, "No RPC error expected: {output:?}")
            }
            TryFromHTTPError::UnknownReason(reason) => {
                write!(f, "Unknown error reason: {reason}")
            }
            TryFromHTTPError::DeserializationError => write!(f, "Failed to deserialize error data"),
            TryFromHTTPError::MissingData => write!(f, "Error data is missing"),
        }
    }
}

impl std::error::Error for TryFromHTTPError {}

impl HTTPError {
    pub fn new(
        code: ErrorCode,
        reason: &str,
        source: Option<Box<dyn std::error::Error>>,
        data: Option<impl Serialize>,
    ) -> HTTPError {
        Self::new_with_severity(code, reason, source, data, Severity::Error)
    }

    pub fn new_with_severity(
        code: ErrorCode,
        reason: &str,
        source: Option<Box<dyn std::error::Error>>,
        data: Option<impl Serialize>,
        severity: Severity,
    ) -> HTTPError {
        HTTPError {
            data: data.and_then(|data| match serde_json::to_value(data) {
                Ok(value) => Some(value),
                Err(err) => {
                    tracing::warn!(
                        ?err,
                        ?source,
                        "Unable to serialize error data for reason: {reason}"
                    );
                    None
                }
            }),
            reason: reason.to_owned(),
            code,
            source,
            severity,
        }
    }

    pub fn internal(err: Box<dyn std::error::Error>) -> HTTPError {
        Self::new(ErrorCode::Internal, "internal", Some(err), None::<()>)
    }

    pub fn not_found(
        reason: &str,
        source: Option<Box<dyn std::error::Error>>,
        data: Option<impl Serialize>,
    ) -> HTTPError {
        Self::new(ErrorCode::NotFound, reason, source, data)
    }

    pub fn bad_request(
        reason: &str,
        source: Option<Box<dyn std::error::Error>>,
        data: Option<impl Serialize>,
    ) -> HTTPError {
        Self::new(ErrorCode::BadRequest, reason, source, data)
    }

    pub fn permission_denied() -> HTTPError {
        Self::new(
            ErrorCode::PermissionDenied,
            "permission-denied",
            None,
            None::<()>,
        )
    }

    /// Get all of the sources of the error
    pub fn sources(&self) -> Vec<&dyn std::error::Error> {
        let mut sources = Vec::new();
        let mut error: &dyn std::error::Error = self;
        while let Some(source) = error.source() {
            sources.push(source.to_owned());
            error = source;
        }
        sources
    }

    /// Get a full report of the error
    pub fn report(&self) -> String {
        let err = self;
        let mut output: String = self.message();

        // Log out each source error
        let mut error: &dyn std::error::Error = err;
        while let Some(source) = error.source() {
            output = format!("{output}\n  Caused by: {source}");
            error = source;
        }
        output
    }

    pub fn message(&self) -> String {
        self.source
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default()
    }
}

impl Display for HTTPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.code, self.message())
    }
}

impl std::error::Error for HTTPError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref())
    }
}

impl actix_web::error::ResponseError for HTTPError {
    fn error_response(&self) -> HttpResponse {
        let error = ErrorOutput {
            error: ErrorDetail {
                code: self.code,
                reason: self.reason.clone(),
                message: self.message(),
                data: self.data.clone(),
            },
        };
        #[allow(clippy::unwrap_used)]
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(serde_json::to_string(&error).unwrap())
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        self.code.status_code()
    }
}

impl From<HTTPError> for ErrorOutput {
    fn from(value: HTTPError) -> Self {
        ErrorOutput {
            error: ErrorDetail {
                code: value.code,
                reason: value.reason.clone(),
                message: value.message(),
                data: value.data.clone(),
            },
        }
    }
}

impl From<eyre::Error> for HTTPError {
    fn from(err: eyre::Error) -> Self {
        HTTPError::new(
            ErrorCode::Internal,
            "internal-error",
            Some(err.into()),
            None::<serde_json::Value>,
        )
    }
}

impl<E> From<Contextful<E>> for HTTPError
where
    E: Into<HTTPError>,
{
    fn from(err: Contextful<E>) -> Self {
        let (_, source) = err.into_parts();
        source.into()
    }
}

pub async fn not_found_error_handler() -> impl Responder {
    let error = HTTPError::new(
        ErrorCode::NotFound, // Assuming you have this variant defined.
        "not-found",
        None,       // No other error caused this error.
        None::<()>, // No extra data.
    );
    error.error_response() // Returns HttpResponse with JSON error.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct FailsSerializationAtRuntime;

    impl Serialize for FailsSerializationAtRuntime {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom(
                "this fails serialisation at runtime",
            ))
        }
    }

    #[test]
    fn test_creating_http_error_from_unserialisable_data_should_return_none_for_data() {
        let err = HTTPError::new(
            ErrorCode::Internal,
            "failed-to-serialize",
            None,
            Some(FailsSerializationAtRuntime),
        );

        assert!(err.data.is_none());
    }
}
