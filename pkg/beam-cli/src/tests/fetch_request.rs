// lint-long-file-override allow-max-lines=300
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mockito::mock;
use reqwest::Url;
use serial_test::serial;

use super::fixtures::test_app_with_output;
use crate::{
    cli::FetchArgs,
    commands::fetch::{
        self, ensure_payment_challenge_transport_for_test, printable_request_header_value_for_test,
    },
    error::Error,
    output::OutputMode,
    runtime::InvocationOverrides,
};

fn x402_v1_fixture() -> &'static str {
    include_str!("fixtures/fetch_x402_v1.json")
}

fn mpp_problem_fixture() -> &'static str {
    include_str!("fixtures/fetch_mpp_problem.json")
}

fn mpp_request_fixture() -> &'static str {
    include_str!("fixtures/fetch_mpp_request.json")
}

#[tokio::test]
#[serial]
async fn fetch_defaults_to_post_when_inline_data_is_present() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let _endpoint = mock("POST", "/submit")
        .match_body("hello")
        .with_status(200)
        .with_body("ok")
        .create();

    fetch::run(
        &app,
        FetchArgs {
            url: format!("{}/submit", mockito::server_url()),
            method: None,
            headers: Vec::new(),
            data: Some("hello".to_string()),
            data_file: None,
            output_path: None,
            verbose: false,
            follow_redirects: false,
            max_redirects: 10,
            connect_timeout: None,
            timeout: None,
            max_fee: None,
            allowed_chains: Vec::new(),
            no_pay: false,
            dev: false,
        },
    )
    .await
    .expect("fetch inline body");
}

#[tokio::test]
#[serial]
async fn fetch_streams_request_body_from_data_file() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let data = tempfile::NamedTempFile::new().expect("create data file");
    std::fs::write(data.path(), "hello from file").expect("write data file");
    let _endpoint = mock("POST", "/submit")
        .match_header("content-length", "15")
        .match_body("hello from file")
        .with_status(200)
        .with_body("ok")
        .create();

    fetch::run(
        &app,
        FetchArgs {
            url: format!("{}/submit", mockito::server_url()),
            method: None,
            headers: Vec::new(),
            data: None,
            data_file: Some(data.path().to_string_lossy().to_string()),
            output_path: None,
            verbose: false,
            follow_redirects: false,
            max_redirects: 10,
            connect_timeout: None,
            timeout: None,
            max_fee: None,
            allowed_chains: Vec::new(),
            no_pay: false,
            dev: false,
        },
    )
    .await
    .expect("fetch file body");
}

#[tokio::test]
#[serial]
async fn fetch_rejects_http_payment_challenge_without_dev_flag() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let challenge_url = format!("{}/paid", mockito::server_url());
    let _endpoint = mock("GET", "/paid")
        .with_status(402)
        .with_header("content-type", "application/json")
        .with_body(x402_v1_fixture())
        .create();

    let err = fetch::run(
        &app,
        FetchArgs {
            url: challenge_url.clone(),
            method: Some("GET".to_string()),
            headers: Vec::new(),
            data: None,
            data_file: None,
            output_path: None,
            verbose: false,
            follow_redirects: false,
            max_redirects: 10,
            connect_timeout: None,
            timeout: None,
            max_fee: None,
            allowed_chains: Vec::new(),
            no_pay: true,
            dev: false,
        },
    )
    .await
    .expect_err("reject insecure payment challenge");

    assert!(matches!(
        err,
        Error::FetchPaymentRequiresHttps { url } if url == challenge_url
    ));
}

#[tokio::test]
#[serial]
async fn fetch_allows_loopback_http_payment_challenge_with_dev_flag() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let _endpoint = mock("GET", "/paid")
        .with_status(402)
        .with_header("content-type", "application/json")
        .with_body(x402_v1_fixture())
        .create();

    let err = fetch::run(
        &app,
        FetchArgs {
            url: format!("{}/paid", mockito::server_url()),
            method: Some("GET".to_string()),
            headers: Vec::new(),
            data: None,
            data_file: None,
            output_path: None,
            verbose: false,
            follow_redirects: false,
            max_redirects: 10,
            connect_timeout: None,
            timeout: None,
            max_fee: None,
            allowed_chains: Vec::new(),
            no_pay: true,
            dev: true,
        },
    )
    .await
    .expect_err("no-pay still exits after parsing challenge");

    assert!(matches!(err, Error::FetchPaymentRequired));
}

#[tokio::test]
#[serial]
async fn fetch_rejects_mpp_retry_when_request_already_has_authorization_header() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let request = URL_SAFE_NO_PAD.encode(mpp_request_fixture().as_bytes());
    let authenticate = format!(
        "Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    );
    let _endpoint = mock("GET", "/paid")
        .match_header("authorization", "Bearer user-token")
        .with_status(402)
        .with_header("content-type", "application/json")
        .with_header("www-authenticate", &authenticate)
        .with_body(mpp_problem_fixture())
        .create();

    let err = fetch::run(
        &app,
        FetchArgs {
            url: format!("{}/paid", mockito::server_url()),
            method: Some("GET".to_string()),
            headers: vec!["Authorization: Bearer user-token".to_string()],
            data: None,
            data_file: None,
            output_path: None,
            verbose: false,
            follow_redirects: false,
            max_redirects: 10,
            connect_timeout: None,
            timeout: None,
            max_fee: None,
            allowed_chains: Vec::new(),
            no_pay: false,
            dev: true,
        },
    )
    .await
    .expect_err("reject conflicting authorization retry");

    assert!(matches!(err, Error::FetchPaymentAuthorizationConflict));
}

#[test]
fn payment_challenge_transport_rejects_remote_http_even_with_dev_flag() {
    let err = ensure_payment_challenge_transport_for_test(
        &fetch_args(true),
        &Url::parse("http://api.example.com/paid").expect("remote challenge url"),
    )
    .expect_err("reject remote http payment challenge");

    assert!(matches!(
        err,
        Error::FetchPaymentRequiresHttps { url } if url == "http://api.example.com/paid"
    ));
}

#[test]
fn payment_challenge_transport_allows_loopback_http_with_dev_flag() {
    ensure_payment_challenge_transport_for_test(
        &fetch_args(true),
        &Url::parse("http://127.0.0.1:8080/paid").expect("loopback challenge url"),
    )
    .expect("allow local http payment challenge");
}

#[test]
fn verbose_request_logging_redacts_sensitive_headers() {
    for header in [
        "Authorization",
        "Proxy-Authorization",
        "Cookie",
        "payment-signature",
        "x-payment",
    ] {
        assert_eq!(
            printable_request_header_value_for_test(header, "secret"),
            "<redacted>",
            "expected {header} to be redacted",
        );
    }
}

#[test]
fn verbose_request_logging_keeps_non_sensitive_headers_visible() {
    assert_eq!(
        printable_request_header_value_for_test("Content-Type", "application/json"),
        "application/json",
    );
}

fn fetch_args(dev: bool) -> FetchArgs {
    FetchArgs {
        url: "https://api.example.com/paid".to_string(),
        method: Some("GET".to_string()),
        headers: Vec::new(),
        data: None,
        data_file: None,
        output_path: None,
        verbose: false,
        follow_redirects: false,
        max_redirects: 10,
        connect_timeout: None,
        timeout: None,
        max_fee: None,
        allowed_chains: Vec::new(),
        no_pay: false,
        dev,
    }
}
