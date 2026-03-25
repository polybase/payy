// lint-long-file-override allow-max-lines=300
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use web3::ethabi::{Token, encode, ethereum_types::U256};

use super::fixtures::{read_rpc_request, test_app_with_output};
use crate::{
    cli::{TokenAction, TokenAddArgs},
    commands::tokens,
    config::ChainRpcConfig,
    error::Error,
    output::OutputMode,
    runtime::{BeamApp, InvocationOverrides},
};

const BASE_CHAIN_ID: u64 = 8453;
const CUSTOM_TOKEN_ADDRESS: &str = "0x0000000000000000000000000000000000000bee";
const DECIMALS_SELECTOR: &str = "313ce567";
const SYMBOL_SELECTOR: &str = "95d89b41";

#[tokio::test]
async fn add_custom_token_rejects_unsupported_manual_decimals_override() {
    let (rpc_url, _selectors, server) =
        spawn_token_metadata_rpc_server("BEAMUSD", MetadataRpcMode::SymbolOnly).await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
    set_base_rpc(&app, &rpc_url).await;

    let err = tokens::run(
        &app,
        Some(TokenAction::Add(TokenAddArgs {
            token: Some(CUSTOM_TOKEN_ADDRESS.to_string()),
            label: Some("BEAMUSD".to_string()),
            decimals: Some(78),
        })),
    )
    .await
    .expect_err("reject unsupported decimals override");
    server.abort();

    assert!(matches!(
        err,
        Error::UnsupportedDecimals {
            decimals: 78,
            max: 77,
        }
    ));
}

#[tokio::test]
async fn add_custom_token_uses_manual_decimals_without_fetching_chain_decimals() {
    let (rpc_url, selectors, server) =
        spawn_token_metadata_rpc_server("BEAMUSD", MetadataRpcMode::SymbolOnly).await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
    set_base_rpc(&app, &rpc_url).await;

    tokens::run(
        &app,
        Some(TokenAction::Add(TokenAddArgs {
            token: Some(CUSTOM_TOKEN_ADDRESS.to_string()),
            label: None,
            decimals: Some(6),
        })),
    )
    .await
    .expect("add custom token with manual decimals");
    server.abort();

    let config = app.config_store.get().await;
    let (_, token) = config
        .known_token_by_label("base", "BEAMUSD")
        .expect("persist token with override decimals");
    assert_eq!(token.decimals, 6);

    let selectors = selectors.lock().expect("rpc selectors").clone();
    assert_eq!(selectors, vec![SYMBOL_SELECTOR.to_string()]);
}

#[tokio::test]
async fn add_custom_token_rejects_unsupported_on_chain_decimals() {
    let (rpc_url, _selectors, server) =
        spawn_token_metadata_rpc_server("BEAMUSD", MetadataRpcMode::SymbolAndDecimals(255)).await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
    set_base_rpc(&app, &rpc_url).await;

    let err = tokens::run(
        &app,
        Some(TokenAction::Add(TokenAddArgs {
            token: Some(CUSTOM_TOKEN_ADDRESS.to_string()),
            label: None,
            decimals: None,
        })),
    )
    .await
    .expect_err("reject unsupported on-chain decimals");
    server.abort();

    assert!(matches!(
        err,
        Error::UnsupportedDecimals {
            decimals: 255,
            max: 77,
        }
    ));
}

#[derive(Clone, Copy)]
enum MetadataRpcMode {
    SymbolOnly,
    SymbolAndDecimals(u8),
}

async fn set_base_rpc(app: &BeamApp, rpc_url: &str) {
    let rpc_url = rpc_url.to_string();
    app.config_store
        .update(move |config| {
            config.rpc_configs.insert(
                "base".to_string(),
                ChainRpcConfig {
                    default_rpc: rpc_url.clone(),
                    rpc_urls: vec![rpc_url.clone()],
                },
            );
            config.tracked_tokens.insert("base".to_string(), Vec::new());
        })
        .await
        .expect("persist base rpc");
}

async fn spawn_token_metadata_rpc_server(
    symbol: &str,
    mode: MetadataRpcMode,
) -> (String, Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind token metadata rpc listener");
    let address = listener.local_addr().expect("listener address");
    let selectors = Arc::new(Mutex::new(Vec::new()));
    let symbol = symbol.to_string();
    let server_selectors = Arc::clone(&selectors);

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_rpc_connection(stream, &symbol, mode, Arc::clone(&server_selectors)).await;
        }
    });

    (format!("http://{address}"), selectors, server)
}

async fn handle_rpc_connection(
    mut stream: TcpStream,
    symbol: &str,
    mode: MetadataRpcMode,
    selectors: Arc<Mutex<Vec<String>>>,
) {
    let request = read_rpc_request(&mut stream).await;
    let body = rpc_response(&request, symbol, mode, selectors);
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

fn rpc_response(
    request: &Value,
    symbol: &str,
    mode: MetadataRpcMode,
    selectors: Arc<Mutex<Vec<String>>>,
) -> String {
    match request["method"].as_str().expect("rpc method") {
        "eth_chainId" => json!({
            "jsonrpc": "2.0",
            "id": request["id"].clone(),
            "result": format!("0x{BASE_CHAIN_ID:x}"),
        })
        .to_string(),
        "eth_call" => {
            let selector =
                request["params"][0]["data"].as_str().expect("call data")[2..10].to_string();
            selectors
                .lock()
                .expect("rpc selectors")
                .push(selector.clone());

            match selector.as_str() {
                SYMBOL_SELECTOR => json!({
                    "jsonrpc": "2.0",
                    "id": request["id"].clone(),
                    "result": encode_string(symbol),
                })
                .to_string(),
                DECIMALS_SELECTOR => match mode {
                    MetadataRpcMode::SymbolOnly => json!({
                        "jsonrpc": "2.0",
                        "id": request["id"].clone(),
                        "error": {
                            "code": -32000,
                            "message": "unexpected decimals lookup",
                        },
                    })
                    .to_string(),
                    MetadataRpcMode::SymbolAndDecimals(decimals) => json!({
                        "jsonrpc": "2.0",
                        "id": request["id"].clone(),
                        "result": encode_uint(decimals),
                    })
                    .to_string(),
                },
                other => json!({
                    "jsonrpc": "2.0",
                    "id": request["id"].clone(),
                    "error": {
                        "code": -32000,
                        "message": format!("unexpected selector {other}"),
                    },
                })
                .to_string(),
            }
        }
        other => json!({
            "jsonrpc": "2.0",
            "id": request["id"].clone(),
            "error": {
                "code": -32000,
                "message": format!("unexpected method {other}"),
            },
        })
        .to_string(),
    }
}

fn encode_string(value: &str) -> String {
    format!(
        "0x{}",
        hex::encode(encode(&[Token::String(value.to_string())]))
    )
}

fn encode_uint(value: u8) -> String {
    format!(
        "0x{}",
        hex::encode(encode(&[Token::Uint(U256::from(value))]))
    )
}
