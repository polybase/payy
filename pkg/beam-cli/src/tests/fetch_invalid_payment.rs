use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mockito::mock;
use serial_test::serial;

use super::fixtures::test_app_with_output;
use crate::{
    cli::FetchArgs, commands::fetch, error::Error, output::OutputMode, runtime::InvocationOverrides,
};

fn mpp_problem_fixture() -> &'static str {
    include_str!("fixtures/fetch_mpp_problem.json")
}

fn mpp_request_fixture() -> &'static str {
    include_str!("fixtures/fetch_mpp_request.json")
}

#[tokio::test]
#[serial]
async fn fetch_rejects_authless_mpp_problem_when_no_pay_is_set() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let _endpoint = mock("GET", "/paid")
        .with_status(402)
        .with_header("content-type", "application/json")
        .with_body(mpp_problem_fixture())
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
    .expect_err("reject malformed mpp response");

    assert!(matches!(err, Error::FetchInvalidPaymentResponse));
}

#[tokio::test]
#[serial]
async fn fetch_rejects_mpp_problem_with_mismatched_auth_challenge_id() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let request = URL_SAFE_NO_PAD.encode(mpp_request_fixture().as_bytes());
    let authenticate = format!(
        "Payment id=\"challenge_456\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    );
    let _endpoint = mock("GET", "/paid")
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
    .expect_err("reject mismatched mpp challenge ids");

    assert!(matches!(err, Error::FetchInvalidPaymentResponse));
}
