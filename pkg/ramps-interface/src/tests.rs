use rpc::{code::ErrorCode, error::HTTPError};

use crate::Error;

#[test]
fn invalid_auth_keeps_legacy_unauthorized_reason() {
    let http_error = HTTPError::from(Error::InvalidAuth);

    assert_eq!(http_error.code, ErrorCode::Unauthenticated);
    assert_eq!(http_error.reason, "unauthorized-to-perform-action");
}

#[test]
fn invalid_admin_key_uses_invalid_admin_key_reason() {
    let http_error = HTTPError::from(Error::InvalidAdminKey);

    assert_eq!(http_error.code, ErrorCode::Unauthenticated);
    assert_eq!(http_error.reason, "invalid-admin-key");
}

#[test]
fn permission_denied_uses_permission_denied_reason() {
    let http_error = HTTPError::from(Error::AdminTokenLacksRequiredScope);

    assert_eq!(http_error.code, ErrorCode::PermissionDenied);
    assert_eq!(http_error.reason, "permission-denied");
    assert_eq!(
        http_error.message(),
        "[ramps-interface] permission denied: admin token lacks required scope"
    );
}
