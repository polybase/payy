use clap::Parser;

use crate::cli::{Cli, Command, FetchArgs};

#[test]
fn parses_fetch_command_flags() {
    let cli = Cli::try_parse_from([
        "beam",
        "fetch",
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json",
        "-H",
        "Accept: application/json",
        "-d",
        "{\"hello\":\"world\"}",
        "-o",
        "response.json",
        "-v",
        "-L",
        "--max-redirects",
        "5",
        "--max-fee",
        "0.01",
        "--allowed-chains",
        "base,8453",
        "https://api.example.com/paid",
    ])
    .expect("parse fetch command");

    assert!(matches!(
        cli.command,
        Some(Command::Fetch(FetchArgs {
            url,
            method,
            headers,
            data,
            output_path,
            verbose,
            follow_redirects,
            max_redirects,
            max_fee,
            allowed_chains,
            no_pay,
            dev,
            ..
        })) if url == "https://api.example.com/paid"
            && method.as_deref() == Some("POST")
            && headers == vec![
                "Content-Type: application/json".to_string(),
                "Accept: application/json".to_string(),
            ]
            && data.as_deref() == Some("{\"hello\":\"world\"}")
            && output_path.as_deref() == Some("response.json")
            && verbose
            && follow_redirects
            && max_redirects == 5
            && max_fee.as_deref() == Some("0.01")
            && allowed_chains == vec!["base".to_string(), "8453".to_string()]
            && !no_pay
            && !dev
    ));

    let cli = Cli::try_parse_from([
        "beam",
        "fetch",
        "--output",
        "response.bin",
        "--no-pay",
        "--dev",
        "https://api.example.com/raw",
    ])
    .expect("parse fetch output long flag");

    assert!(matches!(
        cli.command,
        Some(Command::Fetch(FetchArgs {
            url,
            method,
            output_path,
            no_pay,
            dev,
            ..
        })) if url == "https://api.example.com/raw"
            && method.is_none()
            && output_path.as_deref() == Some("response.bin")
            && no_pay
            && dev
    ));
}

#[test]
fn parses_fetch_request_body_without_implied_method_flag() {
    let cli = Cli::try_parse_from([
        "beam",
        "fetch",
        "-d",
        "hello",
        "https://api.example.com/paid",
    ])
    .expect("parse fetch body");

    assert!(matches!(
        cli.command,
        Some(Command::Fetch(FetchArgs { method, data, .. }))
            if method.is_none() && data.as_deref() == Some("hello")
    ));
}

#[test]
fn parses_private_fetch_payment_flag() {
    let cli = Cli::try_parse_from([
        "beam",
        "fetch",
        "--private-payment",
        "https://api.example.com/paid",
    ])
    .expect("parse private fetch flag");

    assert!(matches!(
        cli.command,
        Some(Command::Fetch(FetchArgs {
            private_payment: true,
            ..
        }))
    ));
}

#[test]
fn rejects_removed_fetch_pay_flag() {
    let err = Cli::try_parse_from(["beam", "fetch", "--pay", "https://api.example.com/paid"])
        .expect_err("reject removed pay flag");

    assert!(err.to_string().contains("--pay"));
}
