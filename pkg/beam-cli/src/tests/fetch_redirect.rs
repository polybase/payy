use reqwest::{StatusCode, Url};

use super::fetch_test_servers::{
    spawn_header_recording_server, spawn_redirect_server, spawn_same_origin_redirect_server,
};
use crate::{
    cli::FetchArgs,
    commands::fetch::{build_initial_request_client_for_test, build_payment_retry_client_for_test},
};

#[tokio::test]
async fn initial_redirect_stops_before_cross_origin_x_api_key_leak() {
    let (destination_url, observed_headers, destination_server) =
        spawn_header_recording_server("x-api-key").await;
    let (origin_url, origin_server) = spawn_redirect_server(destination_url).await;
    let request_url = Url::parse(&origin_url).expect("origin url");
    let client = build_initial_request_client_for_test(&fetch_args(), &request_url)
        .expect("build initial fetch client");

    let response = client
        .get(origin_url)
        .header("x-api-key", "secret")
        .send()
        .await
        .expect("send initial request");

    origin_server.abort();
    destination_server.abort();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(
        observed_headers
            .lock()
            .expect("observed headers")
            .is_empty()
    );
}

#[tokio::test]
async fn initial_redirect_allows_same_origin_x_api_key_redirects() {
    let (origin_url, observed_headers, server) =
        spawn_same_origin_redirect_server("x-api-key").await;
    let request_url = Url::parse(&origin_url).expect("origin url");
    let client = build_initial_request_client_for_test(&fetch_args(), &request_url)
        .expect("build initial fetch client");

    let response = client
        .get(origin_url)
        .header("x-api-key", "secret")
        .send()
        .await
        .expect("send initial request");

    server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        observed_headers
            .lock()
            .expect("observed headers")
            .as_slice(),
        [Some("secret".to_string())]
    );
}

#[tokio::test]
async fn paid_retry_redirect_stops_before_cross_origin_payment_signature_leak() {
    let (destination_url, observed_headers, destination_server) =
        spawn_header_recording_server("payment-signature").await;
    let (origin_url, origin_server) = spawn_redirect_server(destination_url).await;
    let original_url = Url::parse(&origin_url).expect("origin url");
    let client = build_payment_retry_client_for_test(&fetch_args(), &original_url)
        .expect("build restricted retry client");

    let response = client
        .get(origin_url)
        .header("payment-signature", "proof")
        .send()
        .await
        .expect("send retry request");

    origin_server.abort();
    destination_server.abort();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(
        observed_headers
            .lock()
            .expect("observed headers")
            .is_empty()
    );
}

#[tokio::test]
async fn paid_retry_redirect_allows_same_origin_payment_signature_redirects() {
    let (origin_url, observed_headers, server) =
        spawn_same_origin_redirect_server("payment-signature").await;
    let original_url = Url::parse(&origin_url).expect("origin url");
    let client = build_payment_retry_client_for_test(&fetch_args(), &original_url)
        .expect("build restricted retry client");

    let response = client
        .get(origin_url)
        .header("payment-signature", "proof")
        .send()
        .await
        .expect("send retry request");

    server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        observed_headers
            .lock()
            .expect("observed headers")
            .as_slice(),
        [Some("proof".to_string())]
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
