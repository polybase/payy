// lint-long-file-override allow-max-lines=300
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use web3::ethabi::{Token, encode, ethereum_types::U256};

use super::fixtures::test_app_with_output;
use crate::{
    cli::{TokenAction, TokenAddArgs},
    commands::{
        token_report::{TokenBalanceEntry, TokenBalanceReport, render_token_balance_report},
        tokens,
    },
    config::ChainRpcConfig,
    output::OutputMode,
    runtime::{BeamApp, InvocationOverrides},
};

const BASE_CHAIN_ID: u64 = 8453;
const CUSTOM_TOKEN_ADDRESS: &str = "0x0000000000000000000000000000000000000bee";
const DECIMALS_SELECTOR: &str = "313ce567";
const SYMBOL_SELECTOR: &str = "95d89b41";

#[test]
fn render_token_balance_report_includes_tracked_rows() {
    let output = render_token_balance_report(&TokenBalanceReport {
        address: "0x1111111111111111111111111111111111111111".to_string(),
        chain: "base".to_string(),
        native_symbol: "ETH".to_string(),
        rpc_url: "https://beam.example/base".to_string(),
        tokens: vec![
            TokenBalanceEntry {
                balance: "1.25".to_string(),
                decimals: 18,
                is_native: true,
                label: "ETH".to_string(),
                token_address: None,
                value: "1250000000000000000".to_string(),
            },
            TokenBalanceEntry {
                balance: "8".to_string(),
                decimals: 6,
                is_native: false,
                label: "USDC".to_string(),
                token_address: Some("0x833589fcd6edb6e08f4c7c32d4f71b54bda02913".to_string()),
                value: "8000000".to_string(),
            },
        ],
    });

    assert!(
        output
            .default
            .contains("Balances for 0x1111111111111111111111111111111111111111")
    );
    assert!(output.default.contains("ETH"));
    assert!(output.default.contains("USDC"));
    assert_eq!(output.compact.as_deref(), Some("ETH 1.25\nUSDC 8"));
    assert_eq!(output.value["tokens"][0]["is_native"], json!(true));
    assert_eq!(output.value["tokens"][1]["token"], json!("USDC"));
}

#[tokio::test]
async fn add_and_remove_known_token_updates_tracked_state() {
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
    app.config_store
        .update(|config| {
            config.tracked_tokens.insert("base".to_string(), Vec::new());
        })
        .await
        .expect("clear tracked base tokens");

    tokens::run(
        &app,
        Some(TokenAction::Add(TokenAddArgs {
            token: Some("USDC".to_string()),
            label: None,
            decimals: None,
        })),
    )
    .await
    .expect("add known token");

    let config = app.config_store.get().await;
    assert_eq!(
        config.tracked_token_keys_for_chain("base"),
        vec!["USDC".to_string()]
    );
    drop(config);

    tokens::run(
        &app,
        Some(TokenAction::Remove {
            token: "USDC".to_string(),
        }),
    )
    .await
    .expect("remove tracked token");

    let config = app.config_store.get().await;
    assert!(config.tracked_tokens_for_chain("base").is_empty());
    assert!(config.known_token_by_label("base", "USDC").is_some());
}

#[tokio::test]
async fn add_custom_token_uses_chain_metadata_when_label_is_omitted() {
    let (rpc_url, server) = spawn_token_metadata_rpc_server("BEAMUSD", 6).await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
    set_base_rpc(&app, &rpc_url).await;
    app.config_store
        .update(|config| {
            config.tracked_tokens.insert("base".to_string(), Vec::new());
        })
        .await
        .expect("clear tracked base tokens");

    tokens::run(
        &app,
        Some(TokenAction::Add(TokenAddArgs {
            token: Some(CUSTOM_TOKEN_ADDRESS.to_string()),
            label: None,
            decimals: None,
        })),
    )
    .await
    .expect("add custom token");
    server.abort();

    let config = app.config_store.get().await;
    let (_, token) = config
        .known_token_by_label("base", "BEAMUSD")
        .expect("persist looked-up token");

    assert_eq!(token.address, CUSTOM_TOKEN_ADDRESS);
    assert_eq!(token.decimals, 6);
    assert_eq!(
        config.tracked_token_keys_for_chain("base"),
        vec!["BEAMUSD".to_string()]
    );
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
        })
        .await
        .expect("persist base rpc");
}

async fn spawn_token_metadata_rpc_server(
    symbol: &str,
    decimals: u8,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind token metadata rpc listener");
    let address = listener.local_addr().expect("listener address");
    let symbol = symbol.to_string();

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_rpc_connection(stream, &symbol, decimals).await;
        }
    });

    (format!("http://{address}"), server)
}

async fn handle_rpc_connection(mut stream: TcpStream, symbol: &str, decimals: u8) {
    let request = read_rpc_request(&mut stream).await;
    let body = rpc_response(&request, symbol, decimals);
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

async fn read_rpc_request(stream: &mut TcpStream) -> Value {
    let mut buffer = Vec::new();
    let body_offset = loop {
        let mut chunk = [0u8; 1024];
        let read = stream.read(&mut chunk).await.expect("read rpc request");
        assert!(read > 0, "rpc request closed before headers");
        buffer.extend_from_slice(&chunk[..read]);

        if let Some(offset) = header_end(&buffer) {
            break offset;
        }
    };

    let headers = String::from_utf8_lossy(&buffer[..body_offset]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("parse content length"))
        })
        .expect("content-length header");

    let mut body = buffer[body_offset..].to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0u8; content_length - body.len()];
        let read = stream.read(&mut chunk).await.expect("read rpc body");
        assert!(read > 0, "rpc request closed before body");
        body.extend_from_slice(&chunk[..read]);
    }

    serde_json::from_slice(&body[..content_length]).expect("parse rpc body")
}

fn header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn rpc_response(request: &Value, symbol: &str, decimals: u8) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_chainId" => Value::String(format!("0x{BASE_CHAIN_ID:x}")),
        "eth_call" => {
            let call = &request["params"][0];
            let to = call["to"]
                .as_str()
                .expect("eth_call target")
                .to_ascii_lowercase();
            let data = call["data"]
                .as_str()
                .expect("eth_call data")
                .trim_start_matches("0x");

            assert_eq!(to, CUSTOM_TOKEN_ADDRESS.to_ascii_lowercase());
            match &data[..8] {
                SYMBOL_SELECTOR => encode_string(symbol),
                DECIMALS_SELECTOR => encode_uint(decimals),
                selector => panic!("unexpected token metadata selector: {selector}"),
            }
        }
        method => panic!("unexpected rpc method: {method}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}

fn encode_string(value: &str) -> Value {
    Value::String(format!(
        "0x{}",
        hex::encode(encode(&[Token::String(value.to_string())]))
    ))
}

fn encode_uint(value: u8) -> Value {
    Value::String(format!(
        "0x{}",
        hex::encode(encode(&[Token::Uint(U256::from(value))]))
    ))
}
