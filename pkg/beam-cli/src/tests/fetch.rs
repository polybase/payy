// lint-long-file-override allow-max-lines=280
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mockito::mock;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use serial_test::serial;

use super::fixtures::test_app_with_output;
use crate::{
    cli::FetchArgs,
    commands::fetch::{
        self,
        payment::ExecutedPayment,
        protocol::{AmountValue, PaymentChallenge, parse_payment_challenge},
    },
    error::Error,
    output::OutputMode,
    runtime::InvocationOverrides,
};

fn x402_v2_fixture() -> &'static str {
    include_str!("fixtures/fetch_x402_v2.json")
}

fn x402_v1_fixture() -> &'static str {
    include_str!("fixtures/fetch_x402_v1.json")
}

fn mpp_problem_fixture() -> &'static str {
    include_str!("fixtures/fetch_mpp_problem.json")
}

fn mpp_request_fixture() -> &'static str {
    include_str!("fixtures/fetch_mpp_request.json")
}

#[test]
fn parses_x402_v2_challenge_from_payment_required_header() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "payment-required",
        HeaderValue::from_str(
            &base64::engine::general_purpose::STANDARD.encode(x402_v2_fixture().as_bytes()),
        )
        .expect("payment-required header"),
    );

    let challenge = parse_payment_challenge(&headers, b"").expect("parse x402 challenge");

    let Some(PaymentChallenge::X402(challenge)) = challenge else {
        panic!("expected x402 challenge");
    };

    assert_eq!(challenge.version, 2);
    assert_eq!(challenge.offers.len(), 1);
    assert!(matches!(
        challenge.offers[0].amount,
        AmountValue::Atomic(ref value) if value == "10000"
    ));
    assert_eq!(challenge.offers[0].network, "eip155:8453");
}

#[test]
fn parses_x402_v1_challenge_from_body() {
    let challenge = parse_payment_challenge(&HeaderMap::new(), x402_v1_fixture().as_bytes())
        .expect("parse x402 body challenge");

    let Some(PaymentChallenge::X402(challenge)) = challenge else {
        panic!("expected x402 challenge");
    };

    assert_eq!(challenge.version, 1);
    assert_eq!(challenge.offers[0].asset, "native");
    assert!(matches!(
        challenge.offers[0].amount,
        AmountValue::Atomic(ref value) if value == "420000000000000"
    ));
}

#[test]
fn parses_mpp_challenge_from_problem_and_www_authenticate_header() {
    let mut headers = HeaderMap::new();
    let request = URL_SAFE_NO_PAD.encode(mpp_request_fixture().as_bytes());
    let authenticate = format!(
        "Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    );
    headers.insert(
        "www-authenticate",
        HeaderValue::from_str(&authenticate).expect("www-authenticate"),
    );

    let challenge = parse_payment_challenge(&headers, mpp_problem_fixture().as_bytes())
        .expect("parse mpp challenge");

    let Some(PaymentChallenge::Mpp(challenge)) = challenge else {
        panic!("expected mpp challenge");
    };

    assert_eq!(challenge.problem.challenge_id, "challenge_123");
    assert_eq!(
        challenge.auth.as_ref().expect("auth").method,
        "tempo.charge"
    );
    assert_eq!(
        challenge.request.currency,
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
    );
    assert!(matches!(
        challenge.request.amount,
        AmountValue::Human(ref value) if value == "0.01"
    ));
}

#[test]
fn builds_x402_retry_header_payload() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "payment-required",
        HeaderValue::from_str(
            &base64::engine::general_purpose::STANDARD.encode(x402_v2_fixture().as_bytes()),
        )
        .expect("payment-required header"),
    );

    let challenge = parse_payment_challenge(&headers, b"")
        .expect("parse x402 challenge")
        .expect("x402 challenge");
    let PaymentChallenge::X402(challenge) = challenge else {
        panic!("expected x402 challenge");
    };
    let offer = challenge.offers.first().expect("x402 offer");
    let executed = ExecutedPayment {
        accepted: offer.raw.clone(),
        network: offer.network.clone(),
        proof: serde_json::json!({ "txHash": "0xabc123" }),
        scheme: offer.scheme.clone(),
        source: None,
    };

    let header = PaymentChallenge::X402(challenge)
        .retry_header(&executed)
        .expect("build x402 retry header");
    let encoded = header.value.to_str().expect("header value");
    let payload = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("decode x402 payload");
    let payload = serde_json::from_slice::<Value>(&payload).expect("parse x402 payload");

    assert_eq!(header.name.as_str(), "payment-signature");
    assert_eq!(payload["x402Version"], 2);
    assert_eq!(payload["payload"]["txHash"], "0xabc123");
}

#[test]
fn builds_mpp_authorization_header() {
    let mut headers = HeaderMap::new();
    let request = URL_SAFE_NO_PAD.encode(mpp_request_fixture().as_bytes());
    let authenticate = format!(
        "Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    );
    headers.insert(
        "www-authenticate",
        HeaderValue::from_str(&authenticate).expect("www-authenticate"),
    );

    let challenge = parse_payment_challenge(&headers, mpp_problem_fixture().as_bytes())
        .expect("parse mpp challenge")
        .expect("mpp challenge");
    let PaymentChallenge::Mpp(challenge) = challenge else {
        panic!("expected mpp challenge");
    };

    let header = PaymentChallenge::Mpp(challenge)
        .retry_header(&ExecutedPayment {
            accepted: Value::Null,
            network: "eip155:8453".to_string(),
            proof: serde_json::json!({ "hash": "0xabc123", "type": "hash" }),
            scheme: "tempo.charge".to_string(),
            source: Some(
                "did:pkh:eip155:8453:0x4444444444444444444444444444444444444444".to_string(),
            ),
        })
        .expect("build mpp auth header");
    let value = header.value.to_str().expect("authorization value");
    let encoded = value.strip_prefix("Payment ").expect("payment prefix");
    let payload = URL_SAFE_NO_PAD
        .decode(encoded)
        .expect("decode mpp credential");
    let payload = serde_json::from_slice::<Value>(&payload).expect("parse mpp credential");

    assert_eq!(header.name.as_str(), "authorization");
    assert_eq!(payload["payload"]["hash"], "0xabc123");
    assert_eq!(
        payload["source"],
        "did:pkh:eip155:8453:0x4444444444444444444444444444444444444444"
    );
}

#[tokio::test]
#[serial]
async fn fetch_writes_response_to_output_file() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let output = tempfile::NamedTempFile::new().expect("create output file");
    let output_path = output.path().to_string_lossy().to_string();
    let body = "paid-content".repeat(8 * 1024);
    let _endpoint = mock("GET", "/paid")
        .with_status(200)
        .with_body(body.clone())
        .create();

    fetch::run(
        &app,
        FetchArgs {
            url: format!("{}/paid", mockito::server_url()),
            method: Some("GET".to_string()),
            headers: Vec::new(),
            data: None,
            data_file: None,
            output_path: Some(output_path.clone()),
            verbose: false,
            follow_redirects: false,
            max_redirects: 10,
            connect_timeout: None,
            timeout: None,
            max_fee: None,
            allowed_chains: Vec::new(),
            no_pay: false,
            dev: false,
            private_payment: false,
        },
    )
    .await
    .expect("fetch output file");

    let output = std::fs::read_to_string(output_path).expect("read output file");
    assert_eq!(output, body);
}

#[tokio::test]
#[serial]
async fn fetch_returns_payment_required_when_no_pay_is_set() {
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
            private_payment: false,
        },
    )
    .await
    .expect_err("require no-pay failure");

    assert!(matches!(err, Error::FetchPaymentRequired));
}
