use crate::prelude::*;

#[derive(Debug, thiserror::Error, FromContextful)]
pub enum OtherError {
    #[error("[contextful::tests::flattening] foo")]
    Foo,
    #[error("[contextful::tests::flattening] internal error")]
    Internal(#[from] InternalError),
}

#[test]
fn preserves_original_foo_variant_without_wrapping() {
    let err = OtherError::Foo;
    let res = OtherError::from(err.context("new context"));

    assert!(matches!(res, OtherError::Foo));
}

#[test]
fn preserves_original_internal_variant_without_wrapping() {
    let io_err = std::io::Error::other("io error");
    let original = OtherError::from(io_err.context("inner"));

    let res = OtherError::from(original.context("new context"));

    assert!(matches!(res, OtherError::Internal(_)));
}

#[test]
fn internal_preserves_original_context_discarding_new_context() {
    let io_err = std::io::Error::other("io error");
    let err = OtherError::from(io_err.context("original"));

    let res = OtherError::from(err.context("discarded"));

    match res {
        OtherError::Internal(ie) => assert_eq!(ie.context_message(), "original"),
        _ => panic!("Expected Internal variant"),
    }
}
