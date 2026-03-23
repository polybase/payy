use std::convert::TryFrom;

use serde_json::json;

use crate::code::ErrorCode;
use crate::error::{ErrorDetail, ErrorOutput, HTTPError, Severity, TryFromHTTPError};

use super::fixtures::{TestAppError, TestUserData};

#[test]
fn test_unit_variant_conversion() {
    let error = TestAppError::GenericError;
    let http_error = HTTPError::from(error);

    assert_eq!(http_error.code, ErrorCode::BadRequest);
    assert_eq!(http_error.reason, "generic-error");
    assert!(http_error.data.is_none());
    assert!(matches!(http_error.severity, Severity::Error));

    let http_error = HTTPError::new(
        ErrorCode::BadRequest,
        "generic-error",
        None::<Box<dyn std::error::Error>>,
        None::<()>,
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, TestAppError::GenericError);
}

#[test]
fn test_single_unnamed_field_conversion() {
    let user_data = TestUserData {
        id: 42,
        username: "testuser".to_string(),
    };
    let error = TestAppError::UserNotFound(user_data.clone());
    let http_error = HTTPError::from(error);

    assert_eq!(http_error.code, ErrorCode::NotFound);
    assert_eq!(http_error.reason, "user-not-found");
    assert!(http_error.data.is_some());

    let data_value = http_error.data.unwrap();
    let deserialized = serde_json::from_value::<TestUserData>(data_value.clone()).unwrap();
    assert_eq!(deserialized, user_data);

    let http_error = HTTPError::new(
        ErrorCode::NotFound,
        "user-not-found",
        None::<Box<dyn std::error::Error>>,
        Some(data_value),
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, TestAppError::UserNotFound(user_data));
}

#[test]
fn test_multiple_unnamed_fields_conversion() {
    let error = TestAppError::ValidationFailed("email field".to_string(), 400, true);
    let http_error = HTTPError::from(error.clone());

    assert_eq!(http_error.code, ErrorCode::BadRequest);
    assert_eq!(http_error.reason, "validation-failed");
    assert!(http_error.data.is_some());
    assert!(matches!(http_error.severity, Severity::Error));

    let data_value = http_error.data.unwrap();
    let array = serde_json::from_value::<Vec<serde_json::Value>>(data_value.clone()).unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!("email field"));
    assert_eq!(array[1], json!(400));
    assert_eq!(array[2], json!(true));

    let http_error = HTTPError::new(
        ErrorCode::BadRequest,
        "validation-failed",
        None::<Box<dyn std::error::Error>>,
        Some(data_value),
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, error);
}

#[test]
fn test_warn_severity_override() {
    let error = TestAppError::WarnSeverity;
    let http_error = HTTPError::from(error);

    assert_eq!(http_error.code, ErrorCode::BadRequest);
    assert_eq!(http_error.reason, "warn-severity");
    assert!(http_error.data.is_none());
    assert!(matches!(http_error.severity, Severity::Warn));

    let http_error = HTTPError::new(
        ErrorCode::BadRequest,
        "warn-severity",
        None::<Box<dyn std::error::Error>>,
        None::<()>,
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, TestAppError::WarnSeverity);
}

#[test]
fn test_from_error_output() {
    let error_output = ErrorOutput {
        error: ErrorDetail {
            code: ErrorCode::AlreadyExists,
            reason: "duplicate-user".to_string(),
            message: "User already exists".to_string(),
            data: Some(json!({
                "id": 777,
                "email": "output@example.com"
            })),
        },
    };

    let result = TestAppError::try_from(error_output);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        TestAppError::DuplicateUser {
            id: 777,
            email: "output@example.com".to_string(),
        }
    );
}

#[test]
fn test_missing_data_error() {
    let http_error = HTTPError::new(
        ErrorCode::NotFound,
        "user-not-found",
        None::<Box<dyn std::error::Error>>,
        None::<()>,
    );

    let result = TestAppError::try_from(http_error);
    assert!(result.is_err());
    match result.unwrap_err() {
        TryFromHTTPError::MissingData => {}
        _ => panic!("Expected MissingData error"),
    }
}

#[test]
fn test_unknown_reason_error() {
    let http_error = HTTPError::new(
        ErrorCode::BadRequest,
        "unknown-error-code",
        None::<Box<dyn std::error::Error>>,
        None::<()>,
    );

    let result = TestAppError::try_from(http_error);
    assert!(result.is_err());
    match result.unwrap_err() {
        TryFromHTTPError::UnknownReason(reason) => {
            assert_eq!(reason, "unknown-error-code");
        }
        _ => panic!("Expected UnknownReason error"),
    }
}

#[test]
fn test_deserialization_error() {
    let http_error = HTTPError::new(
        ErrorCode::NotFound,
        "user-not-found",
        None::<Box<dyn std::error::Error>>,
        Some(json!("invalid_data_type")),
    );

    let result = TestAppError::try_from(http_error);
    assert!(result.is_err());
    match result.unwrap_err() {
        TryFromHTTPError::DeserializationError => {}
        _ => panic!("Expected DeserializationError"),
    }
}
