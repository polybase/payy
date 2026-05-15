// lint-long-file-override allow-max-lines=300
use reqwest::{
    Method, StatusCode, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue},
};

use super::fetch_test_servers::{
    request_body, request_header, request_method, spawn_header_recording_server,
    spawn_recording_redirect_server, spawn_recording_redirect_server_with_status,
    spawn_request_recording_server,
};
use crate::{
    cli::FetchArgs,
    commands::fetch::{
        protocol::RetryHeader, send_retry_request_for_test, send_retry_request_with_spec_for_test,
    },
    error::Error,
};

#[tokio::test]
async fn paid_retry_request_uses_challenged_url_for_payment_signature() {
    let (destination_url, destination_headers, destination_server) =
        spawn_header_recording_server("payment-signature").await;
    let (origin_url, origin_paths, origin_server) =
        spawn_recording_redirect_server(destination_url.clone()).await;
    let request_url = Url::parse(&format!("{origin_url}/start")).expect("request url");
    let challenged_url = Url::parse(&format!("{destination_url}/paid")).expect("challenged url");

    let response = send_retry_request_for_test(
        &fetch_args(),
        &request_url,
        &challenged_url,
        RetryHeader {
            name: HeaderName::from_static("payment-signature"),
            value: HeaderValue::from_static("proof"),
        },
    )
    .await
    .expect("send retry request");

    origin_server.abort();
    destination_server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(origin_paths.lock().expect("origin paths").is_empty());
    assert_eq!(
        destination_headers
            .lock()
            .expect("destination headers")
            .as_slice(),
        [Some("proof".to_string())]
    );
}

#[tokio::test]
async fn paid_retry_request_uses_challenged_url_for_authorization() {
    let (destination_url, destination_headers, destination_server) =
        spawn_header_recording_server("authorization").await;
    let (origin_url, origin_paths, origin_server) =
        spawn_recording_redirect_server(destination_url.clone()).await;
    let request_url = Url::parse(&format!("{origin_url}/start")).expect("request url");
    let challenged_url = Url::parse(&format!("{destination_url}/paid")).expect("challenged url");

    let response = send_retry_request_for_test(
        &fetch_args(),
        &request_url,
        &challenged_url,
        RetryHeader {
            name: HeaderName::from_static("authorization"),
            value: HeaderValue::from_static("Payment proof"),
        },
    )
    .await
    .expect("send retry request");

    origin_server.abort();
    destination_server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(origin_paths.lock().expect("origin paths").is_empty());
    assert_eq!(
        destination_headers
            .lock()
            .expect("destination headers")
            .as_slice(),
        [Some("Payment proof".to_string())]
    );
}

#[tokio::test]
async fn cross_origin_payment_signature_retry_drops_original_request_metadata() {
    let (destination_url, destination_requests, destination_server) =
        spawn_request_recording_server().await;
    let (origin_url, origin_paths, origin_server) =
        spawn_recording_redirect_server(destination_url.clone()).await;
    let request_url = Url::parse(&format!("{origin_url}/start")).expect("request url");
    let challenged_url = Url::parse(&format!("{destination_url}/paid")).expect("challenged url");

    let response = send_retry_request_with_spec_for_test(
        &fetch_args(),
        &request_url,
        &challenged_url,
        Method::POST,
        request_headers(),
        Some(b"hello".to_vec()),
        RetryHeader {
            name: HeaderName::from_static("payment-signature"),
            value: HeaderValue::from_static("proof"),
        },
    )
    .await
    .expect("send retry request");

    origin_server.abort();
    destination_server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(origin_paths.lock().expect("origin paths").is_empty());

    let requests = destination_requests.lock().expect("destination requests");
    let request = requests.first().expect("recorded request");
    assert_eq!(request_method(request), "GET");
    assert_eq!(
        request_header(request, "payment-signature"),
        Some("proof".to_string())
    );
    assert_eq!(request_header(request, "authorization"), None);
    assert_eq!(request_header(request, "cookie"), None);
    assert_eq!(request_header(request, "x-api-key"), None);
    assert_eq!(request_header(request, "content-type"), None);
    assert!(request_body(request).is_empty());
}

#[tokio::test]
async fn cross_origin_authorization_retry_drops_original_request_metadata() {
    let (destination_url, destination_requests, destination_server) =
        spawn_request_recording_server().await;
    let (origin_url, origin_paths, origin_server) =
        spawn_recording_redirect_server(destination_url.clone()).await;
    let request_url = Url::parse(&format!("{origin_url}/start")).expect("request url");
    let challenged_url = Url::parse(&format!("{destination_url}/paid")).expect("challenged url");

    let response = send_retry_request_with_spec_for_test(
        &fetch_args(),
        &request_url,
        &challenged_url,
        Method::POST,
        request_headers(),
        Some(b"hello".to_vec()),
        RetryHeader {
            name: HeaderName::from_static("authorization"),
            value: HeaderValue::from_static("Payment proof"),
        },
    )
    .await
    .expect("send retry request");

    origin_server.abort();
    destination_server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(origin_paths.lock().expect("origin paths").is_empty());

    let requests = destination_requests.lock().expect("destination requests");
    let request = requests.first().expect("recorded request");
    assert_eq!(request_method(request), "GET");
    assert_eq!(
        request_header(request, "authorization"),
        Some("Payment proof".to_string())
    );
    assert_eq!(request_header(request, "payment-signature"), None);
    assert_eq!(request_header(request, "cookie"), None);
    assert_eq!(request_header(request, "x-api-key"), None);
    assert_eq!(request_header(request, "content-type"), None);
    assert!(request_body(request).is_empty());
}

#[tokio::test]
async fn same_origin_authorization_retry_rejects_existing_authorization_header() {
    let (origin_url, origin_requests, origin_server) = spawn_request_recording_server().await;
    let request_url = Url::parse(&format!("{origin_url}/start")).expect("request url");
    let challenged_url = Url::parse(&format!("{origin_url}/paid")).expect("challenged url");

    let err = send_retry_request_with_spec_for_test(
        &fetch_args(),
        &request_url,
        &challenged_url,
        Method::POST,
        request_headers(),
        Some(b"hello".to_vec()),
        RetryHeader {
            name: HeaderName::from_static("authorization"),
            value: HeaderValue::from_static("Payment proof"),
        },
    )
    .await
    .expect_err("reject conflicting authorization retry");

    origin_server.abort();

    assert!(matches!(err, Error::FetchPaymentAuthorizationConflict));
    assert!(origin_requests.lock().expect("origin requests").is_empty());
}

#[tokio::test]
async fn same_origin_authorization_retry_stops_before_cross_origin_redirect() {
    let (destination_url, destination_requests, destination_server) =
        spawn_request_recording_server().await;
    let (origin_url, origin_paths, origin_server) = spawn_recording_redirect_server_with_status(
        destination_url.clone(),
        StatusCode::TEMPORARY_REDIRECT.as_u16(),
        "Temporary Redirect",
    )
    .await;
    let request_url = Url::parse(&format!("{origin_url}/start")).expect("request url");
    let challenged_url = Url::parse(&format!("{origin_url}/paid")).expect("challenged url");

    let response = send_retry_request_with_spec_for_test(
        &fetch_args(),
        &request_url,
        &challenged_url,
        Method::POST,
        request_headers_without_authorization(),
        Some(b"hello".to_vec()),
        RetryHeader {
            name: HeaderName::from_static("authorization"),
            value: HeaderValue::from_static("Payment proof"),
        },
    )
    .await
    .expect("send retry request");

    origin_server.abort();
    destination_server.abort();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        origin_paths.lock().expect("origin paths").as_slice(),
        ["/paid".to_string()]
    );
    assert!(
        destination_requests
            .lock()
            .expect("destination requests")
            .is_empty()
    );
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

fn request_headers_without_authorization() -> HeaderMap {
    let mut headers = request_headers();
    headers.remove(AUTHORIZATION);
    headers
}

fn request_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_static("Bearer user-token"),
    );
    headers.insert(
        HeaderName::from_static("cookie"),
        HeaderValue::from_static("session=abc"),
    );
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
