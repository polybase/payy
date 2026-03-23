use parse_link::{ParseError, parse_url};
use serde::Serialize;
use std::env;

const INVALID_LINK_MESSAGE: &str = "Invalid Link";

#[derive(Serialize)]
struct Output {
    link: Option<parse_link::Link>,
    error: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let url = args
        .get(1)
        .map_or(String::new(), |value| value.trim().to_owned());

    if url.is_empty() {
        emit(Output {
            link: None,
            error: Some(INVALID_LINK_MESSAGE.to_owned()),
        });
        return;
    }

    match parse_url(&url) {
        Ok(link) => emit(Output {
            link: Some(link),
            error: None,
        }),
        Err(err) => emit(Output {
            link: None,
            error: Some(map_parse_error(&err).to_owned()),
        }),
    }
}

fn emit(output: Output) {
    println!(
        "{}",
        serde_json::to_string(&output).expect("failed to serialize parse_link_cli output")
    );
}

fn map_parse_error(err: &ParseError) -> &'static str {
    match err {
        ParseError::EmptyInput
        | ParseError::MissingPrefix
        | ParseError::UnknownPrefix(_)
        | ParseError::MissingSegment(_)
        | ParseError::MissingHash(_)
        | ParseError::InvalidElement { .. }
        | ParseError::InvalidRequestFormat
        | ParseError::InvalidSendFormat => INVALID_LINK_MESSAGE,
    }
}
