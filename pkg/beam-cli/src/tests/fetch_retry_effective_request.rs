use reqwest::{
    Method, StatusCode, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};

use super::fetch_test_servers::{
    request_body, request_header, request_method,
    spawn_same_origin_redirect_challenge_server_with_status,
};
use crate::{
    cli::FetchArgs,
    commands::fetch::{
        protocol::RetryHeader, send_request_for_test, send_retry_request_with_spec_for_test,
    },
};

#[tokio::test]
async fn same_origin_302_payment_challenge_retries_effective_get_request() {
    let (request_url, challenged_requests, server) =
        spawn_same_origin_redirect_challenge_server_with_status(
            StatusCode::FOUND.as_u16(),
            "Found",
        )
        .await;
    let request_url = Url::parse(&request_url).expect("request url");

    let sent = send_request_for_test(
        &fetch_args(),
        &request_url,
        Method::POST,
        request_headers(),
        Some(b"hello".to_vec()),
    )
    .await
    .expect("send initial request");

    assert_eq!(sent.status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(sent.effective_spec.method, Method::GET);
    assert_eq!(
        sent.effective_spec.url,
        request_url.join("/paid").expect("challenged url")
    );
    assert_eq!(sent.effective_spec.body, None);
    assert!(!sent.effective_spec.headers.contains_key("content-type"));
    assert!(!sent.effective_spec.headers.contains_key("content-length"));

    let response = send_retry_request_with_spec_for_test(
        &fetch_args(),
        &sent.effective_spec.url,
        &sent.effective_spec.url,
        sent.effective_spec.method.clone(),
        sent.effective_spec.headers.clone(),
        sent.effective_spec.body.clone(),
        RetryHeader {
            name: HeaderName::from_static("payment-signature"),
            value: HeaderValue::from_static("proof"),
        },
    )
    .await
    .expect("send retry request");

    server.abort();

    assert_eq!(response.status(), StatusCode::OK);

    let challenged_requests = challenged_requests.lock().expect("challenged requests");
    assert_eq!(challenged_requests.len(), 2);

    let initial_request = challenged_requests
        .first()
        .expect("initial challenged request");
    assert_eq!(request_method(initial_request), "GET");
    assert_eq!(
        request_header(initial_request, "x-api-key"),
        Some("secret".to_string())
    );
    assert_eq!(request_header(initial_request, "content-type"), None);
    assert!(request_body(initial_request).is_empty());

    let retry_request = challenged_requests
        .get(1)
        .expect("retry challenged request");
    assert_eq!(request_method(retry_request), "GET");
    assert_eq!(
        request_header(retry_request, "x-api-key"),
        Some("secret".to_string())
    );
    assert_eq!(
        request_header(retry_request, "payment-signature"),
        Some("proof".to_string())
    );
    assert_eq!(request_header(retry_request, "content-type"), None);
    assert!(request_body(retry_request).is_empty());
}

#[tokio::test]
async fn same_origin_303_payment_challenge_uses_effective_get_request() {
    let (request_url, challenged_requests, server) =
        spawn_same_origin_redirect_challenge_server_with_status(
            StatusCode::SEE_OTHER.as_u16(),
            "See Other",
        )
        .await;
    let request_url = Url::parse(&request_url).expect("request url");

    let sent = send_request_for_test(
        &fetch_args(),
        &request_url,
        Method::PUT,
        request_headers(),
        Some(b"hello".to_vec()),
    )
    .await
    .expect("send initial request");

    server.abort();

    assert_eq!(sent.status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(sent.effective_spec.method, Method::GET);
    assert_eq!(
        sent.effective_spec.url,
        request_url.join("/paid").expect("challenged url")
    );
    assert_eq!(sent.effective_spec.body, None);
    assert!(!sent.effective_spec.headers.contains_key("content-type"));
    assert!(!sent.effective_spec.headers.contains_key("content-length"));

    let challenged_requests = challenged_requests.lock().expect("challenged requests");
    assert_eq!(challenged_requests.len(), 1);

    let challenged_request = challenged_requests.first().expect("challenged request");
    assert_eq!(request_method(challenged_request), "GET");
    assert_eq!(
        request_header(challenged_request, "x-api-key"),
        Some("secret".to_string())
    );
    assert_eq!(request_header(challenged_request, "content-type"), None);
    assert!(request_body(challenged_request).is_empty());
}

fn fetch_args() -> FetchArgs {
    FetchArgs {
        url: "https://api.example.com/paid".to_string(),
        method: Some("GET".to_string()),
        headers: Vec::new(),
        data: None,
        data_file: None,
        output_path: None,
        verbose: false,
        follow_redirects: true,
        max_redirects: 10,
        connect_timeout: None,
        timeout: None,
        max_fee: None,
        allowed_chains: Vec::new(),
        no_pay: false,
        dev: false,
        private_payment: false,
    }
}

fn request_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-api-key"),
        HeaderValue::from_static("secret"),
    );
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("text/plain"),
    );
    headers
}
