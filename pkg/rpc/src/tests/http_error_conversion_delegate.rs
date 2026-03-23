use std::convert::TryFrom;

use crate::code::ErrorCode;
use crate::error::HTTPError;

use super::fixtures::{SubError, SubSubError, TestAppError};

#[test]
fn test_delegate_conversion_roundtrip() {
    let sub_error = SubError::PermissionDenied;
    let error = TestAppError::Delegated(sub_error.clone());
    let http_error = HTTPError::from(error.clone());

    assert_eq!(http_error.code, ErrorCode::PermissionDenied);
    assert_eq!(http_error.reason, "sub-permission-denied");
    assert!(http_error.data.is_none());

    let http_error = HTTPError::new(
        ErrorCode::PermissionDenied,
        "sub-permission-denied",
        None::<Box<dyn std::error::Error>>,
        None::<()>,
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, error);
}

#[test]
fn test_delegate_multi_depth_conversion_roundtrip() {
    let sub_error = SubSubError::PermissionIsReallyDenied;
    let delegated = SubError::SubDelegate(sub_error);
    let error = TestAppError::Delegated(delegated);
    let http_error = HTTPError::from(error.clone());

    assert_eq!(http_error.code, ErrorCode::PermissionDenied);
    assert_eq!(http_error.reason, "sub-sub-permission-denied");
    assert!(http_error.data.is_none());

    let http_error = HTTPError::new(
        ErrorCode::PermissionDenied,
        "sub-sub-permission-denied",
        None::<Box<dyn std::error::Error>>,
        None::<()>,
    );
    let recovered = TestAppError::try_from(http_error).unwrap();
    assert_eq!(recovered, error);
}
