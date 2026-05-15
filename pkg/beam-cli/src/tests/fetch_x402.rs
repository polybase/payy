// lint-long-file-override allow-max-lines=300
use contracts::U256;
use serde_json::Value;
use tokio::{io::AsyncWriteExt, net::TcpListener};

use super::fixtures::{read_rpc_request, test_app};
use crate::{
    cli::FetchArgs,
    commands::fetch::{
        payment::prepare_x402_payment,
        protocol::{AmountValue, X402Challenge, X402Offer},
    },
    config::ChainRpcConfig,
    error::Error,
    keystore::{KeyStore, StoredKdf, StoredWallet},
    runtime::{BeamApp, InvocationOverrides},
};

const TEST_WALLET_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
const RECIPIENT_ADDRESS: &str = "0x3333333333333333333333333333333333333333";

#[tokio::test]
async fn prepare_x402_payment_selects_offer_allowed_by_chain_allowlist() {
    let (ethereum_rpc, ethereum_server) = spawn_x402_offer_rpc_server(1, U256::exp10(18)).await;
    let (base_rpc, base_server) = spawn_x402_offer_rpc_server(8453, U256::exp10(18)).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_rpc_config(&app, "ethereum", &ethereum_rpc).await;
    set_rpc_config(&app, "base", &base_rpc).await;

    let payment = prepare_x402_payment(
        &app,
        &fetch_args(None, &["base"]),
        &x402_challenge(vec![
            native_offer("eip155:1", "100000000000000000"),
            native_offer("eip155:8453", "100000000000000000"),
        ]),
    )
    .await
    .expect("prepare x402 payment");

    ethereum_server.abort();
    base_server.abort();

    assert_eq!(payment.chain.key, "base");
}

#[tokio::test]
async fn prepare_x402_payment_skips_offers_without_sufficient_balance() {
    let (ethereum_rpc, ethereum_server) =
        spawn_x402_offer_rpc_server(1, U256::from(100_000_000_000_000_000u64)).await;
    let (base_rpc, base_server) =
        spawn_x402_offer_rpc_server(8453, U256::from(2_000_000_000_000_000_000u64)).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_rpc_config(&app, "ethereum", &ethereum_rpc).await;
    set_rpc_config(&app, "base", &base_rpc).await;

    let payment = prepare_x402_payment(
        &app,
        &fetch_args(None, &[]),
        &x402_challenge(vec![
            native_offer("eip155:1", "500000000000000000"),
            native_offer("eip155:8453", "100000000000000000"),
        ]),
    )
    .await
    .expect("prepare x402 payment");

    ethereum_server.abort();
    base_server.abort();

    assert_eq!(payment.chain.key, "base");
}

#[tokio::test]
async fn prepare_x402_payment_skips_offers_above_max_fee() {
    let (base_rpc, base_server) = spawn_x402_offer_rpc_server(8453, U256::exp10(18)).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_rpc_config(&app, "base", &base_rpc).await;

    let payment = prepare_x402_payment(
        &app,
        &fetch_args(Some("0.11"), &[]),
        &x402_challenge(vec![
            native_offer("eip155:8453", "150000000000000000"),
            native_offer("eip155:8453", "100000000000000000"),
        ]),
    )
    .await
    .expect("prepare x402 payment");

    base_server.abort();

    assert_eq!(payment.chain.key, "base");
    assert_eq!(payment.amount, U256::from(100_000_000_000_000_000u64));
}

#[tokio::test]
async fn prepare_x402_payment_returns_max_fee_error_when_every_offer_is_too_expensive() {
    let (base_rpc, base_server) = spawn_x402_offer_rpc_server(8453, U256::exp10(18)).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_rpc_config(&app, "base", &base_rpc).await;

    let err = prepare_x402_payment(
        &app,
        &fetch_args(Some("0.11"), &[]),
        &x402_challenge(vec![
            native_offer("eip155:8453", "150000000000000000"),
            native_offer("eip155:8453", "120000000000000000"),
        ]),
    )
    .await
    .expect_err("reject over-cap offers");

    base_server.abort();

    assert!(matches!(err, Error::FetchPaymentExceedsMaxFee));
}

#[tokio::test]
async fn prepare_x402_payment_returns_chain_not_allowed_when_allowlist_filters_every_offer() {
    let (base_rpc, base_server) = spawn_x402_offer_rpc_server(8453, U256::exp10(18)).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_rpc_config(&app, "base", &base_rpc).await;

    let err = prepare_x402_payment(
        &app,
        &fetch_args(None, &["ethereum"]),
        &x402_challenge(vec![native_offer("eip155:8453", "100000000000000000")]),
    )
    .await
    .expect_err("reject disallowed chains");

    base_server.abort();

    match err {
        Error::FetchPaymentChainNotAllowed { chain } => {
            assert_eq!(chain, "Base (8453)");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn prepare_x402_payment_returns_chain_mismatch_when_selector_filters_every_offer() {
    let (_temp_dir, app) = test_app(InvocationOverrides {
        chain: Some("ethereum".to_string()),
        ..InvocationOverrides::default()
    })
    .await;

    let err = prepare_x402_payment(
        &app,
        &fetch_args(None, &[]),
        &x402_challenge(vec![
            native_offer("eip155:8453", "100000000000000000"),
            native_offer("eip155:137", "100000000000000000"),
        ]),
    )
    .await
    .expect_err("reject mismatched explicit chain");

    assert!(matches!(
        err,
        Error::FetchPaymentChainMismatch { challenge, selected }
            if challenge == "Base (8453), Polygon (137)"
                && selected == "Ethereum (1)"
    ));
}

fn fetch_args(max_fee: Option<&str>, allowed_chains: &[&str]) -> FetchArgs {
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
        max_fee: max_fee.map(ToString::to_string),
        allowed_chains: allowed_chains.iter().map(ToString::to_string).collect(),
        no_pay: false,
        dev: false,
        private_payment: false,
    }
}

fn native_offer(network: &str, amount: &str) -> X402Offer {
    X402Offer {
        amount: AmountValue::Atomic(amount.to_string()),
        asset: "native".to_string(),
        network: network.to_string(),
        pay_to: RECIPIENT_ADDRESS.to_string(),
        private_address: None,
        raw: Value::Null,
        scheme: "exact".to_string(),
    }
}

fn x402_challenge(offers: Vec<X402Offer>) -> X402Challenge {
    X402Challenge {
        offers,
        resource: None,
        version: 2,
    }
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

async fn set_rpc_config(app: &BeamApp, chain_key: &str, rpc_url: &str) {
    let rpc_url = rpc_url.to_string();
    app.config_store
        .update(move |config| {
            config.rpc_configs.insert(
                chain_key.to_string(),
                ChainRpcConfig {
                    default_rpc: rpc_url.clone(),
                    rpc_urls: vec![rpc_url.clone()],
                },
            );
        })
        .await
        .expect("persist rpc config");
}

async fn spawn_x402_offer_rpc_server(
    chain_id: u64,
    native_balance: U256,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind x402 offer rpc listener");
    let address = listener.local_addr().expect("listener address");

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept rpc connection");
            let request = read_rpc_request(&mut stream).await;
            let body = x402_offer_rpc_response(&request, chain_id, native_balance);
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

    (format!("http://{address}"), server)
}

fn x402_offer_rpc_response(request: &Value, chain_id: u64, native_balance: U256) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_chainId" => serde_json::to_value(U256::from(chain_id)).expect("chain id"),
        "eth_estimateGas" => serde_json::to_value(U256::from(21_000u64)).expect("estimate gas"),
        "eth_gasPrice" => serde_json::to_value(U256::from(1_000_000_000u64)).expect("gas price"),
        "eth_getBalance" => serde_json::to_value(native_balance).expect("native balance"),
        other => panic!("unexpected rpc method {other}"),
    };

    serde_json::json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}
