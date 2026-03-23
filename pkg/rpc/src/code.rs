use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, Display, EnumString, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ErrorCode {
    BadRequest,
    InvalidArgument,
    FailedPrecondition,
    OutOfRange,
    Unauthenticated,
    PermissionDenied,
    NotFound,
    Aborted,
    AlreadyExists,
    ResourceExhausted,
    Cancelled,
    PayloadTooLarge,
    Unavailable,
    Internal,
    DeadlineExceeded,
}

impl ErrorCode {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
            ErrorCode::InvalidArgument => StatusCode::BAD_REQUEST,
            ErrorCode::FailedPrecondition => StatusCode::BAD_REQUEST,
            ErrorCode::OutOfRange => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
            ErrorCode::PermissionDenied => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Aborted => StatusCode::CONFLICT,
            ErrorCode::AlreadyExists => StatusCode::CONFLICT,
            ErrorCode::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::Cancelled => StatusCode::NOT_ACCEPTABLE,
            ErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
        }
    }
}
