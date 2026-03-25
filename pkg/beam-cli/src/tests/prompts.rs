use std::io::Cursor;

use crate::{
    error::Error,
    prompts::{prompt_required_with, prompt_with_default_with},
};

#[test]
fn prompt_required_retries_after_blank_input() {
    let mut input = Cursor::new("\nbeam-dev\n");
    let mut output = Vec::new();

    let value =
        prompt_required_with("beam chain name", &mut input, &mut output).expect("read prompt");

    assert_eq!(value, "beam-dev");
    assert_eq!(
        String::from_utf8(output).expect("decode prompt"),
        "beam chain name: beam chain name: "
    );
}

#[test]
fn prompt_required_errors_when_input_is_closed() {
    let mut input = Cursor::new("");
    let mut output = Vec::new();

    let err = prompt_required_with("beam chain name", &mut input, &mut output)
        .expect_err("closed stdin should not spin forever");

    assert!(matches!(
        err,
        Error::PromptClosed { label } if label == "beam chain name"
    ));
    assert_eq!(
        String::from_utf8(output).expect("decode prompt"),
        "beam chain name: "
    );
}

#[test]
fn prompt_with_default_uses_default_when_input_is_empty() {
    let mut input = Cursor::new("\n");
    let mut output = Vec::new();

    let value =
        prompt_with_default_with("beam chain native symbol", "ETH", &mut input, &mut output)
            .expect("resolve default prompt value");

    assert_eq!(value, "ETH");
    assert_eq!(
        String::from_utf8(output).expect("decode prompt"),
        "beam chain native symbol [ETH]: "
    );
}

#[test]
fn prompt_with_default_errors_when_input_is_closed() {
    let mut input = Cursor::new("");
    let mut output = Vec::new();

    let err = prompt_with_default_with("beam chain native symbol", "ETH", &mut input, &mut output)
        .expect_err("closed stdin should not accept the default");

    assert!(matches!(
        err,
        Error::PromptClosed { label } if label == "beam chain native symbol"
    ));
    assert_eq!(
        String::from_utf8(output).expect("decode prompt"),
        "beam chain native symbol [ETH]: "
    );
}
