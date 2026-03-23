// lint-long-file-override allow-max-lines=300

mod flattening;

use std::io;

use serde_json::json;

use crate::{Contextful, ErrorContextExt, FromContextful, InternalError, ResultContextExt};

#[derive(Debug, thiserror::Error)]
#[error("[contextful::tests] demo error")]
struct Demo;

#[derive(Debug, thiserror::Error)]
#[error("[contextful::tests] other error")]
struct Other;

#[derive(Debug, thiserror::Error)]
enum Top {
    #[error("[contextful::tests] demo error with context: {0}")]
    Demo(#[from] Contextful<Demo>),
}

#[test]
fn it_wraps_and_formats() {
    let res = Err::<(), Demo>(Demo);
    let err = Top::from(res.context("doing a thing").unwrap_err());
    let s = err.to_string();
    assert!(s.contains("doing a thing"));
    assert!(s.contains("demo error"));
}

#[test]
fn test_deref() {
    let res = Err::<(), Demo>(Demo);
    let err = res.context("test context").unwrap_err();
    // Should be able to access Demo through deref
    let _demo: &Demo = &err;
}

#[test]
fn test_with_context_lazy() {
    let mut called = false;
    let res = Err::<(), Demo>(Demo);
    let _err = res
        .with_context(|| {
            called = true;
            "lazy context".to_string()
        })
        .unwrap_err();
    assert!(called);
}

#[test]
fn test_with_context_not_called_on_ok() {
    let mut called = false;
    let res = Ok::<i32, Demo>(42);
    let val = res
        .with_context(|| {
            called = true;
            "lazy context".to_string()
        })
        .unwrap();
    assert!(!called);
    assert_eq!(val, 42);
}

#[test]
fn error_and_result_context_exts_work() {
    let res = Err::<(), Demo>(Demo);
    let err = res
        .with_context(|| "result context".to_owned())
        .unwrap_err();

    let wrapped = err.context("error context");
    assert_eq!(wrapped.context_message(), "error context");

    let without_context = Demo.without_context();
    assert_eq!(without_context.context_message(), "");
}

#[test]
fn serde_preserves_context_and_handles_legacy() {
    let err = Contextful::<u32>::new("ctx", 5);
    let value = serde_json::to_value(&err).unwrap();
    assert_eq!(value, json!({ "context": "ctx", "source": 5 }));

    let with_context = serde_json::from_value::<Contextful<u32>>(value).unwrap();
    assert_eq!(with_context.context_message(), "ctx");
    assert_eq!(*with_context, 5);

    let legacy = serde_json::from_value::<Contextful<u32>>(json!(5)).unwrap();
    assert_eq!(legacy.context_message(), "");
    assert_eq!(*legacy, 5);
}

#[test]
fn wrap_err_adds_context() {
    let err = Demo.wrap_err("wrap err eager");
    assert_eq!(err.context_message(), "wrap err eager");
    assert_eq!(
        err.to_string(),
        "wrap err eager: [contextful::tests] demo error"
    );
}

#[test]
fn wrap_err_with_invokes_closure_once() {
    let mut calls = 0;
    let err = Demo.wrap_err_with(|| {
        calls += 1;
        "lazy message".to_string()
    });
    assert_eq!(calls, 1);
    assert_eq!(err.context_message(), "lazy message");
    assert_eq!(
        err.to_string(),
        "lazy message: [contextful::tests] demo error"
    );
}

#[test]
fn io_error_with_context_adds_context() {
    let err = io::Error::other("io error");
    let wrapped = err.with_context(|| "io context".to_owned());
    assert_eq!(wrapped.context_message(), "io context");
    assert!(wrapped.to_string().contains("io error"));
}

#[test]
fn internal_error_downcast_preserves_context() {
    let res = Err::<(), Other>(Other);
    let err = res.context("other context").unwrap_err();
    let internal = InternalError::from(err);
    // SAFETY: We just constructed the InternalError, so there's only one reference.
    let downcasted = unsafe { internal.downcast::<Other>() }.expect("expected downcast to succeed");
    assert_eq!(downcasted.context_message(), "other context");
}

#[derive(Debug, thiserror::Error, FromContextful)]
#[contextful(map_demo_error)]
enum TestError {
    #[error("[contextful::tests] mapped demo error")]
    Demo(Contextful<Demo>),
    #[error("[contextful::tests] internal")]
    Internal(#[from] InternalError),
}

fn map_demo_error(err: Contextful<Demo>) -> TestError {
    TestError::Demo(err)
}

#[test]
fn test_contextful_map_error() {
    let res = Err::<(), Demo>(Demo);
    let ctx_err = res.context("foo").unwrap_err();
    let err = TestError::from(ctx_err);

    match err {
        TestError::Demo(d) => {
            assert_eq!(d.context_message(), "foo");
        }
        _ => panic!("wrong variant"),
    }
}

#[derive(Debug, thiserror::Error, FromContextful)]
#[contextful(map_demo_error_multi)]
#[contextful(map_other_error)]
enum MultiMapError {
    #[error("[contextful::tests] mapped demo error")]
    Demo(Contextful<Demo>),
    #[error("[contextful::tests] mapped other error")]
    Other(Contextful<Other>),
    #[error("[contextful::tests] internal")]
    Internal(#[from] InternalError),
}

fn map_other_error(err: Contextful<Other>) -> MultiMapError {
    MultiMapError::Other(err)
}

fn map_demo_error_multi(err: Contextful<Demo>) -> MultiMapError {
    MultiMapError::Demo(err)
}

#[test]
fn test_contextful_multiple_maps() {
    let res = Err::<(), Other>(Other);
    let ctx_err = res.context("other context").unwrap_err();
    let err = MultiMapError::from(ctx_err);

    match err {
        MultiMapError::Other(other) => {
            assert_eq!(other.context_message(), "other context");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn internal_error_is_clone() {
    let res = Err::<(), Other>(Other);
    let err = res.context("clone context").unwrap_err();
    let internal = InternalError::from(err);

    let cloned = internal.clone();

    assert_eq!(internal.context_message(), "clone context");
    assert_eq!(cloned.context_message(), "clone context");
    assert!(internal.to_string().contains("other error"));
    assert!(cloned.to_string().contains("other error"));
}

#[test]
fn internal_error_downcast_fails_with_multiple_refs() {
    let res = Err::<(), Other>(Other);
    let err = res.context("multi ref context").unwrap_err();
    let internal = InternalError::from(err);

    // Clone creates another reference
    let _cloned = internal.clone();

    // SAFETY: This is intentionally testing the failure case when there are
    // multiple references. The downcast should fail and return the error.
    let result = unsafe { internal.downcast::<Other>() };
    assert!(result.is_err(), "downcast should fail with multiple refs");

    let returned_err = result.unwrap_err();
    assert_eq!(returned_err.context_message(), "multi ref context");
}

#[test]
fn internal_error_downcast_ref_works() {
    let res = Err::<(), Other>(Other);
    let err = res.context("ref context").unwrap_err();
    let internal = InternalError::from(err);

    // downcast_ref is safe and works even with clones
    let cloned = internal.clone();

    let ref1 = internal.downcast_ref::<Other>();
    let ref2 = cloned.downcast_ref::<Other>();

    assert!(ref1.is_some(), "downcast_ref should succeed on original");
    assert!(ref2.is_some(), "downcast_ref should succeed on clone");

    // Wrong type returns None
    let wrong_type = internal.downcast_ref::<Demo>();
    assert!(
        wrong_type.is_none(),
        "downcast_ref to wrong type should fail"
    );
}

#[derive(Debug, thiserror::Error)]
#[error("[contextful::tests] wrapper error")]
struct Wrapper {
    #[source]
    inner: Other,
}

#[test]
fn internal_error_recursive_downcast_ref_works() {
    // Create a nested error: InternalError -> Wrapper -> Other
    let wrapper = Wrapper { inner: Other };
    let res = Err::<(), Wrapper>(wrapper);
    let err = res.context("wrapper context").unwrap_err();
    let internal = InternalError::from(err);

    // Direct downcast_ref finds Wrapper
    assert!(internal.downcast_ref::<Wrapper>().is_some());
    // But not Other (it's nested)
    assert!(internal.downcast_ref::<Other>().is_none());

    // recursive_downcast_ref finds both Wrapper and nested Other
    assert!(internal.recursive_downcast_ref::<Wrapper>().is_some());
    assert!(internal.recursive_downcast_ref::<Other>().is_some());

    // Wrong type still returns None
    assert!(internal.recursive_downcast_ref::<Demo>().is_none());
}
