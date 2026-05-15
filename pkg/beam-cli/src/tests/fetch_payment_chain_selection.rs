// lint-long-file-override allow-max-lines=260
use contracts::U256;
use serde_json::{Value, json};
use tokio::{io::AsyncWriteExt, net::TcpListener};

use super::fixtures::{read_rpc_request, test_app};
use crate::{
    chains::{BeamChains, ConfiguredChain},
    cli::FetchArgs,
    commands::fetch::{
        payment::{prepare_mpp_payment, prepare_x402_payment},
        protocol::{
            AmountValue, MppAuthChallenge, MppChallenge, MppPaymentRequest, MppProblem,
            X402Challenge, X402Offer,
        },
    },
    config::ChainRpcConfig,
    keystore::{KeyStore, StoredKdf, StoredWallet},
    runtime::{BeamApp, InvocationOverrides},
};

const BASE_CHAIN_ID: u64 = 8453;
const STALE_CHAIN_ID: u64 = 31_337;
const RECIPIENT_ADDRESS: &str = "0x3333333333333333333333333333333333333333";
const STALE_CHAIN_KEY: &str = "forgotten-chain";
const TEST_WALLET_ADDRESS: &str = "0x1111111111111111111111111111111111111111";

#[tokio::test]
async fn prepare_x402_payment_ignores_stale_default_chain_rpc_when_offer_chain_is_resolvable() {
    let (base_rpc, server) = spawn_payment_prepare_rpc_server(BASE_CHAIN_ID, U256::exp10(18)).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_stale_default_chain(&app).await;
    set_rpc_config(&app, "base", &base_rpc).await;

    let payment = prepare_x402_payment(&app, &fetch_args(), &x402_challenge(BASE_CHAIN_ID))
        .await
        .expect("prepare x402 payment with stale default chain");
    server.abort();

    assert_eq!(payment.chain.key, "base");
    assert_eq!(
        payment
            .selected_chain
            .as_ref()
            .map(|chain| chain.key.as_str()),
        Some(STALE_CHAIN_KEY)
    );
    assert_eq!(
        payment.selected_chain.as_ref().map(|chain| chain.chain_id),
        Some(STALE_CHAIN_ID)
    );
}

#[tokio::test]
async fn prepare_mpp_payment_ignores_stale_default_chain_rpc_when_request_chain_is_resolvable() {
    let (base_rpc, server) = spawn_payment_prepare_rpc_server(BASE_CHAIN_ID, U256::exp10(18)).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_default_wallet(&app).await;
    set_stale_default_chain(&app).await;
    set_rpc_config(&app, "base", &base_rpc).await;

    let payment = prepare_mpp_payment(&app, &mpp_challenge(BASE_CHAIN_ID))
        .await
        .expect("prepare mpp payment with stale default chain");
    server.abort();

    assert_eq!(payment.chain.key, "base");
    assert_eq!(
        payment
            .selected_chain
            .as_ref()
            .map(|chain| chain.key.as_str()),
        Some(STALE_CHAIN_KEY)
    );
    assert_eq!(
        payment.selected_chain.as_ref().map(|chain| chain.chain_id),
        Some(STALE_CHAIN_ID)
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
        follow_redirects: false,
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

fn x402_challenge(chain_id: u64) -> X402Challenge {
    X402Challenge {
        offers: vec![X402Offer {
            amount: AmountValue::Atomic("100000000000000000".to_string()),
            asset: "native".to_string(),
            network: format!("eip155:{chain_id}"),
            pay_to: RECIPIENT_ADDRESS.to_string(),
            private_address: None,
            raw: Value::Null,
            scheme: "exact".to_string(),
        }],
        resource: None,
        version: 2,
    }
}

fn mpp_challenge(chain_id: u64) -> MppChallenge {
    MppChallenge {
        auth: Some(MppAuthChallenge {
            description: None,
            digest: None,
            expires: None,
            id: "challenge_123".to_string(),
            intent: "charge".to_string(),
            method: "tempo.charge".to_string(),
            opaque: None,
            realm: "api.example.com".to_string(),
            request: "request".to_string(),
        }),
        problem: MppProblem {
            challenge_id: "challenge_123".to_string(),
            detail: Some("Tempo test charge".to_string()),
            title: None,
        },
        request: MppPaymentRequest {
            amount: AmountValue::Human("0.01".to_string()),
            chain_id: Some(chain_id),
            currency: "native".to_string(),
            description: Some("Tempo test charge".to_string()),
            recipient: RECIPIENT_ADDRESS.to_string(),
            private_address: None,
        },
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

async fn set_stale_default_chain(app: &BeamApp) {
    app.chain_store
        .set(BeamChains {
            chains: vec![ConfiguredChain {
                aliases: Vec::new(),
                chain_id: STALE_CHAIN_ID,
                name: "Forgotten Chain".to_string(),
                native_symbol: "FGT".to_string(),
                privacy: None,
            }],
        })
        .await
        .expect("persist custom chains");

    app.config_store
        .update(|config| {
            config.default_chain = STALE_CHAIN_KEY.to_string();
            config.rpc_configs.remove(STALE_CHAIN_KEY);
        })
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

async fn spawn_payment_prepare_rpc_server(
    chain_id: u64,
    native_balance: U256,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind payment prepare rpc listener");
    let address = listener.local_addr().expect("listener address");

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept rpc connection");
            let request = read_rpc_request(&mut stream).await;
            let body = payment_prepare_rpc_response(&request, chain_id, native_balance);
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

fn payment_prepare_rpc_response(request: &Value, chain_id: u64, native_balance: U256) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_chainId" => serde_json::to_value(U256::from(chain_id)).expect("chain id"),
        "eth_estimateGas" => serde_json::to_value(U256::from(21_000u64)).expect("estimate gas"),
        "eth_gasPrice" => serde_json::to_value(U256::from(1_000_000_000u64)).expect("gas price"),
        "eth_getBalance" => serde_json::to_value(native_balance).expect("native balance"),
        other => panic!("unexpected rpc method {other}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}
