// lint-long-file-override allow-max-lines=500
use std::sync::{Arc, Mutex};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use contracts::U256;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};

use super::fixtures::{read_rpc_request, test_app};
use crate::{
    commands::fetch::{
        payment::{PaymentAssetKind, PreparedPayment, prepare_mpp_payment},
        protocol::{MppChallenge, PaymentChallenge, parse_payment_challenge},
    },
    error::Error,
    evm::parse_units,
    keystore::{KeyStore, StoredKdf, StoredWallet},
    runtime::{BeamApp, InvocationOverrides},
};

const TEST_WALLET_ADDRESS: &str = "0x1111111111111111111111111111111111111111";

fn mpp_problem_fixture() -> &'static str {
    include_str!("fixtures/fetch_mpp_problem.json")
}

fn mpp_request_fixture() -> &'static str {
    include_str!("fixtures/fetch_mpp_request.json")
}

fn mpp_request_without_chain_fixture(currency: &str, decimals: Option<u8>) -> String {
    let decimals = decimals
        .map(|value| format!(",\n  \"decimals\": {value}"))
        .unwrap_or_default();

    format!(
        r#"{{
  "amount": "0.01",
  "currency": "{currency}",
  "recipient": "0x3333333333333333333333333333333333333333"{decimals},
  "description": "Tempo test charge"
}}"#
    )
}

fn mpp_native_request_fixture() -> &'static str {
    r#"{
  "amount": "0.01",
  "currency": "native",
  "recipient": "0x3333333333333333333333333333333333333333",
  "chainId": 8453,
  "description": "Tempo test charge"
}"#
}

fn mpp_unknown_token_request_fixture() -> &'static str {
    r#"{
  "amount": "0.01",
  "currency": "0x0000000000000000000000000000000000000bee",
  "decimals": 18,
  "recipient": "0x3333333333333333333333333333333333333333",
  "chainId": 8453,
  "description": "Tempo test charge"
}"#
}

fn parse_mpp_challenge_with_header(authenticate: String) -> MppChallenge {
    let mut headers = HeaderMap::new();
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

    *challenge
}

async fn seed_default_wallet(app: &BeamApp) {
    app.keystore_store
        .set(KeyStore {
            wallets: vec![StoredWallet {
                address: TEST_WALLET_ADDRESS.to_string(),
                encrypted_key: "encrypted-key".to_string(),
                name: "alice".to_string(),
                salt: "salt".to_string(),
                kdf: StoredKdf::default(),
            }],
        })
        .await
        .expect("persist keystore");

    app.config_store
        .update(|config| config.default_wallet = Some("alice".to_string()))
        .await
        .expect("persist default wallet");
}

async fn prepare_rpc_only_mpp_payment(request_body: &str, chain_id: u64) -> PreparedPayment {
    let (rpc_url, server) = spawn_payment_prepare_rpc_server(chain_id).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        rpc: Some(rpc_url),
        ..InvocationOverrides::default()
    })
    .await;
    seed_default_wallet(&app).await;

    let request = URL_SAFE_NO_PAD.encode(request_body.as_bytes());
    let challenge = parse_mpp_challenge_with_header(format!(
        "Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    ));

    let payment = prepare_mpp_payment(&app, &challenge)
        .await
        .expect("prepare mpp payment");
    server.abort();
    payment
}

async fn spawn_payment_prepare_rpc_server(chain_id: u64) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind payment prepare rpc listener");
    let address = listener.local_addr().expect("listener address");

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_payment_prepare_rpc_connection(stream, chain_id).await;
        }
    });

    (format!("http://{address}"), server)
}

async fn handle_payment_prepare_rpc_connection(mut stream: TcpStream, chain_id: u64) {
    let request = read_rpc_request(&mut stream).await;
    let body = payment_prepare_rpc_response(&request, chain_id);
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .await
        .expect("write rpc response");
}

fn payment_prepare_rpc_response(request: &Value, chain_id: u64) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_chainId" => serde_json::to_value(U256::from(chain_id)).expect("chain id"),
        "eth_estimateGas" => serde_json::to_value(U256::from(21_000u64)).expect("estimate gas"),
        "eth_gasPrice" => serde_json::to_value(U256::from(1_000_000_000u64)).expect("gas price"),
        other => panic!("unexpected rpc method {other}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}

async fn spawn_token_prepare_rpc_server(
    chain_id: u64,
    decimals: u8,
) -> (String, Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind token prepare rpc listener");
    let address = listener.local_addr().expect("listener address");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let server_methods = Arc::clone(&methods);

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept rpc connection");
            let request = read_rpc_request(&mut stream).await;
            server_methods
                .lock()
                .expect("record rpc method")
                .push(request["method"].as_str().expect("rpc method").to_string());
            let body = token_prepare_rpc_response(&request, chain_id, decimals);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write rpc response");
        }
    });

    (format!("http://{address}"), methods, server)
}

fn token_prepare_rpc_response(request: &Value, chain_id: u64, decimals: u8) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_call" => Value::String(format!("0x{decimals:064x}")),
        "eth_chainId" => serde_json::to_value(U256::from(chain_id)).expect("chain id"),
        "eth_estimateGas" => serde_json::to_value(U256::from(65_000u64)).expect("estimate gas"),
        "eth_gasPrice" => serde_json::to_value(U256::from(1_000_000_000u64)).expect("gas price"),
        other => panic!("unexpected rpc method {other}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}

#[test]
fn parses_mpp_challenge_from_lowercase_payment_scheme() {
    let request = URL_SAFE_NO_PAD.encode(mpp_request_fixture().as_bytes());
    let challenge = parse_mpp_challenge_with_header(format!(
        "payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    ));

    assert_eq!(challenge.problem.challenge_id, "challenge_123");
    assert_eq!(
        challenge.auth.as_ref().expect("auth").method,
        "tempo.charge"
    );
}

#[test]
fn parses_payment_challenge_from_multi_scheme_www_authenticate_header() {
    let request = URL_SAFE_NO_PAD.encode(mpp_request_fixture().as_bytes());
    let challenge = parse_mpp_challenge_with_header(format!(
        "Basic realm=\"api.example.com\", Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    ));

    assert_eq!(challenge.problem.challenge_id, "challenge_123");
    assert_eq!(
        challenge.auth.as_ref().expect("auth").realm,
        "api.example.com"
    );
}

#[tokio::test]
async fn prepare_mpp_payment_requires_explicit_chain_when_challenge_omits_it() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let request = URL_SAFE_NO_PAD.encode(
        mpp_request_without_chain_fixture("0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", Some(6))
            .as_bytes(),
    );
    let challenge = parse_mpp_challenge_with_header(format!(
        "Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    ));

    let err = prepare_mpp_payment(&app, &challenge)
        .await
        .expect_err("require explicit chain");

    assert!(matches!(err, Error::FetchPaymentChainRequired));
}

#[tokio::test]
async fn prepare_mpp_payment_rejects_explicit_chain_that_disagrees_with_challenge() {
    let (rpc_url, server) = spawn_payment_prepare_rpc_server(8453).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        chain: Some("ethereum".to_string()),
        rpc: Some(rpc_url),
        ..InvocationOverrides::default()
    })
    .await;
    seed_default_wallet(&app).await;

    let request = URL_SAFE_NO_PAD.encode(mpp_native_request_fixture().as_bytes());
    let challenge = parse_mpp_challenge_with_header(format!(
        "Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    ));

    let err = prepare_mpp_payment(&app, &challenge)
        .await
        .expect_err("reject mismatched explicit chain");
    server.abort();

    assert!(matches!(
        err,
        Error::FetchPaymentChainMismatch { challenge, selected }
            if challenge == "Base (8453)" && selected == "Ethereum (1)"
    ));
}

#[tokio::test]
async fn prepare_mpp_payment_accepts_matching_explicit_chain() {
    let (rpc_url, server) = spawn_payment_prepare_rpc_server(8453).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        chain: Some("base".to_string()),
        rpc: Some(rpc_url),
        ..InvocationOverrides::default()
    })
    .await;
    seed_default_wallet(&app).await;

    let request = URL_SAFE_NO_PAD.encode(mpp_native_request_fixture().as_bytes());
    let challenge = parse_mpp_challenge_with_header(format!(
        "Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    ));

    let payment = prepare_mpp_payment(&app, &challenge)
        .await
        .expect("prepare mpp payment");
    server.abort();

    assert_eq!(payment.chain.chain_id, 8453);
    assert_eq!(payment.chain.key, "base");
    assert_eq!(payment.network, "eip155:8453");
    assert_eq!(
        payment.selected_chain.as_ref().map(|chain| chain.chain_id),
        Some(8453)
    );
    assert_eq!(
        payment
            .selected_chain
            .as_ref()
            .map(|chain| chain.key.as_str()),
        Some("base")
    );
}

#[tokio::test]
async fn prepare_mpp_payment_rehydrates_known_chain_metadata_for_token_labels_over_rpc_only() {
    let payment =
        prepare_rpc_only_mpp_payment(&mpp_request_without_chain_fixture("USDC", None), 137).await;

    assert_eq!(payment.chain.chain_id, 137);
    assert_eq!(payment.chain.key, "polygon");
    assert_eq!(payment.chain.native_symbol, "MATIC");
    assert_eq!(payment.asset.decimals, 6);
    assert_eq!(payment.asset.label, "USDC");
    assert!(matches!(
        payment.asset.kind,
        PaymentAssetKind::Erc20(address)
            if format!("{address:#x}") == "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359"
    ));
}

#[tokio::test]
async fn prepare_mpp_payment_rehydrates_known_chain_native_symbol_over_rpc_only() {
    let payment =
        prepare_rpc_only_mpp_payment(&mpp_request_without_chain_fixture("MATIC", None), 137).await;

    assert_eq!(payment.chain.chain_id, 137);
    assert_eq!(payment.chain.key, "polygon");
    assert_eq!(payment.chain.native_symbol, "MATIC");
    assert_eq!(payment.asset.label, "MATIC");
    assert!(matches!(payment.asset.kind, PaymentAssetKind::Native));

    let confirmation = payment.confirmation_message("MPP");
    assert!(confirmation.contains("Network: Polygon (137)"));
    assert!(confirmation.contains("Estimated gas:"));
    assert!(confirmation.contains("MATIC"));
}

#[tokio::test]
async fn prepare_mpp_payment_fetches_unknown_token_decimals_from_contract() {
    let (rpc_url, methods, server) = spawn_token_prepare_rpc_server(8453, 6).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        rpc: Some(rpc_url),
        ..InvocationOverrides::default()
    })
    .await;
    seed_default_wallet(&app).await;

    let request = URL_SAFE_NO_PAD.encode(mpp_unknown_token_request_fixture().as_bytes());
    let challenge = parse_mpp_challenge_with_header(format!(
        "Payment id=\"challenge_123\", realm=\"api.example.com\", method=\"tempo.charge\", intent=\"charge\", request=\"{request}\""
    ));

    let payment = prepare_mpp_payment(&app, &challenge)
        .await
        .expect("prepare mpp payment");
    server.abort();

    assert_eq!(payment.asset.decimals, 6);
    assert_eq!(
        payment.amount,
        parse_units("0.01", 6).expect("scale token amount with on-chain decimals")
    );
    assert!(
        methods
            .lock()
            .expect("rpc methods")
            .iter()
            .any(|method| method == "eth_call")
    );
}
