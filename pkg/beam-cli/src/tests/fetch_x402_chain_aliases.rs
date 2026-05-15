// lint-long-file-override allow-max-lines=220
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
    keystore::{KeyStore, StoredKdf, StoredWallet},
    runtime::{BeamApp, InvocationOverrides},
};

const TEST_WALLET_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
const RECIPIENT_ADDRESS: &str = "0x3333333333333333333333333333333333333333";

#[tokio::test]
async fn prepare_x402_payment_accepts_alias_offer_for_normalized_chain_selector() {
    let (payy_dev_rpc, payy_dev_server) = spawn_x402_offer_rpc_server(7297).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        chain: Some("payy_dev".to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    seed_default_wallet(&app).await;
    set_rpc_config(&app, "payy-dev", &payy_dev_rpc).await;

    let payment = prepare_x402_payment(
        &app,
        &fetch_args(&[]),
        &x402_challenge(vec![native_offer("payydev")]),
    )
    .await
    .expect("prepare x402 payment for selector alias");

    payy_dev_server.abort();

    assert_eq!(payment.chain.key, "payy-dev");
    assert_eq!(payment.network, "payydev");
}

#[tokio::test]
async fn prepare_x402_payment_accepts_alias_offer_for_chain_allowlist() {
    let (bnb_rpc, bnb_server) = spawn_x402_offer_rpc_server(56).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_rpc_config(&app, "bnb", &bnb_rpc).await;

    let payment = prepare_x402_payment(
        &app,
        &fetch_args(&["bnb"]),
        &x402_challenge(vec![native_offer("bsc")]),
    )
    .await
    .expect("prepare x402 payment for allowlist alias");

    bnb_server.abort();

    assert_eq!(payment.chain.key, "bnb");
    assert_eq!(payment.network, "bsc");
}

#[tokio::test]
async fn prepare_x402_payment_prefers_default_chain_when_offer_uses_alias() {
    let (base_rpc, base_server) = spawn_x402_offer_rpc_server(8453).await;
    let (arbitrum_rpc, arbitrum_server) = spawn_x402_offer_rpc_server(42161).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_default_chain(&app, "arbitrum").await;
    set_rpc_config(&app, "base", &base_rpc).await;
    set_rpc_config(&app, "arbitrum", &arbitrum_rpc).await;

    let payment = prepare_x402_payment(
        &app,
        &fetch_args(&[]),
        &x402_challenge(vec![native_offer("eip155:8453"), native_offer("arb")]),
    )
    .await
    .expect("prepare x402 payment for preferred alias");

    base_server.abort();
    arbitrum_server.abort();

    assert_eq!(payment.chain.key, "arbitrum");
    assert_eq!(payment.network, "arb");
}

fn fetch_args(allowed_chains: &[&str]) -> FetchArgs {
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
        allowed_chains: allowed_chains.iter().map(ToString::to_string).collect(),
        no_pay: false,
        dev: false,
        private_payment: false,
    }
}

fn native_offer(network: &str) -> X402Offer {
    X402Offer {
        amount: AmountValue::Atomic("100000000000000000".to_string()),
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

async fn set_default_chain(app: &BeamApp, default_chain: &str) {
    let default_chain = default_chain.to_string();
    app.config_store
        .update(move |config| config.default_chain = default_chain.clone())
        .await
        .expect("persist default chain");
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

async fn spawn_x402_offer_rpc_server(chain_id: u64) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind x402 offer rpc listener");
    let address = listener.local_addr().expect("listener address");

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept rpc connection");
            let request = read_rpc_request(&mut stream).await;
            let body = x402_offer_rpc_response(&request, chain_id);
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

fn x402_offer_rpc_response(request: &Value, chain_id: u64) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_chainId" => serde_json::to_value(U256::from(chain_id)).expect("chain id"),
        "eth_estimateGas" => serde_json::to_value(U256::from(21_000u64)).expect("estimate gas"),
        "eth_gasPrice" => serde_json::to_value(U256::from(1_000_000_000u64)).expect("gas price"),
        "eth_getBalance" => serde_json::to_value(U256::exp10(18)).expect("native balance"),
        other => panic!("unexpected rpc method {other}"),
    };

    serde_json::json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}
