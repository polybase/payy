use std::convert::TryFrom;

use serde_json::json;

use crate::code::ErrorCode;
use crate::error::HTTPError;

use super::fixtures::{
    ComplexErrorData, DuplicateUserData, TestAppError, ValidationDetails, ValidationFailedData,
};

#[test]
fn test_named_fields_conversion() {
    let error = TestAppError::DuplicateUser {
        id: 999,
        email: "test@example.com".to_string(),
    };
    let http_error = HTTPError::from(error.clone());

    assert_eq!(http_error.code, ErrorCode::AlreadyExists);
    assert_eq!(http_error.reason, "duplicate-user");
    assert!(http_error.data.is_some());

    let data_value = http_error.data.unwrap();
    assert_eq!(
        data_value,
        json!({
            "id": 999,
            "email": "test@example.com"
        })
    );

    let http_error = HTTPError::new(
        ErrorCode::AlreadyExists,
        "duplicate-user",
        None::<Box<dyn std::error::Error>>,
        Some(data_value),
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, error);
}

#[test]
fn test_complex_named_fields_with_optionals() {
    let error = TestAppError::ComplexError {
        code: -100,
        active: false,
        details: Some("Additional context".to_string()),
        metadata: Some(ValidationDetails {
            field: "username".to_string(),
            message: "Too short".to_string(),
        }),
    };
    let http_error = HTTPError::from(error.clone());

    assert_eq!(http_error.code, ErrorCode::FailedPrecondition);
    assert_eq!(http_error.reason, "complex-error");
    assert!(http_error.data.is_some());

    let data_value = http_error.data.unwrap();
    let expected = json!({
        "code": -100,
        "active": false,
        "details": "Additional context",
        "metadata": {
            "field": "username",
            "message": "Too short"
        }
    });
    assert_eq!(data_value, expected);

    let http_error = HTTPError::new(
        ErrorCode::FailedPrecondition,
        "complex-error",
        None::<Box<dyn std::error::Error>>,
        Some(data_value),
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, error);
}

#[test]
fn test_complex_named_fields_with_none_values() {
    let error = TestAppError::ComplexError {
        code: 50,
        active: true,
        details: None,
        metadata: None,
    };
    let http_error = HTTPError::from(error.clone());

    let data_value = http_error.data.unwrap();
    assert_eq!(
        data_value,
        json!({
            "code": 50,
            "active": true,
            "details": null,
            "metadata": null
        })
    );

    let http_error = HTTPError::new(
        ErrorCode::FailedPrecondition,
        "complex-error",
        None::<Box<dyn std::error::Error>>,
        Some(data_value),
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, error);
}

#[test]
fn test_generated_structs() {
    let validation_data = ValidationFailedData("test".to_string(), 123, false);
    let json_value = serde_json::to_value(&validation_data).unwrap();
    let deserialized = serde_json::from_value::<ValidationFailedData>(json_value).unwrap();
    assert_eq!(deserialized.0, "test");
    assert_eq!(deserialized.1, 123);
    assert!(!deserialized.2);

    let user_data = DuplicateUserData {
        id: 456,
        email: "struct@test.com".to_string(),
    };
    let json_value = serde_json::to_value(&user_data).unwrap();
    let deserialized = serde_json::from_value::<DuplicateUserData>(json_value).unwrap();
    assert_eq!(deserialized.id, 456);
    assert_eq!(deserialized.email, "struct@test.com");

    let complex_data = ComplexErrorData {
        code: 789,
        active: true,
        details: None,
        metadata: Some(ValidationDetails {
            field: "test".to_string(),
            message: "msg".to_string(),
        }),
    };
    let json_value = serde_json::to_value(&complex_data).unwrap();
    let deserialized = serde_json::from_value::<ComplexErrorData>(json_value).unwrap();
    assert_eq!(deserialized.code, 789);
    assert!(deserialized.active);
    assert_eq!(deserialized.details, None);
    assert!(deserialized.metadata.is_some());
}
