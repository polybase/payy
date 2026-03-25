use std::io;

use crate::{error::Error, keystore::prompt_secret_with};

#[test]
fn interrupted_secret_prompts_return_error_interrupted() {
    let err = prompt_secret_with(
        || Err(io::Error::new(io::ErrorKind::Interrupted, "ctrl-c")),
        "read beam password",
    )
    .expect_err("ctrl-c should map to interrupted");

    assert!(matches!(err, Error::Interrupted));
}
