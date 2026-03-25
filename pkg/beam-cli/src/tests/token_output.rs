use contracts::Client;
use serde_json::{Value, json};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use web3::ethabi::{Token, encode};

use super::fixtures::read_rpc_request;
use crate::{
    commands::{
        erc20::render_balance_output,
        token_report::{TokenBalanceEntry, TokenBalanceReport, render_token_balance_report},
        tokens::lookup_token_label,
    },
    human_output::sanitize_control_chars,
    runtime::parse_address,
    table::{render_markdown_table, render_table},
};

const SYMBOL_SELECTOR: &str = "95d89b41";
const TOKEN_ADDRESS: &str = "0x0000000000000000000000000000000000000bee";

#[test]
fn sanitize_control_chars_rewrites_terminal_control_bytes() {
    assert_eq!(
        sanitize_control_chars("beam\nusd\t\x1b[31m"),
        "beam usd ?[31m"
    );
}

#[test]
fn render_table_sanitizes_control_characters_in_cells() {
    let rendered = render_table(&["token"], &[vec!["BEAM\nUSD\x1b[31m".to_string()]]);

    assert!(!rendered.contains('\x1b'));
    assert_eq!(
        rendered.lines().nth(2).expect("table row").trim_end(),
        "BEAM USD?[31m"
    );
}

#[test]
fn render_markdown_table_escapes_pipes_and_backslashes() {
    let rendered = render_markdown_table(
        &["token"],
        &[
            vec![r"BEAM|USD\vault".to_string()],
            vec!["line\nbreak".to_string()],
        ],
    );

    assert_eq!(
        rendered,
        "| token |\n| --- |\n| BEAM\\|USD\\\\vault |\n| line break |"
    );
}

#[test]
fn render_token_balance_report_sanitizes_human_facing_token_labels() {
    let label = "BEAM\nUSD|\x1b[31m";
    let output = render_token_balance_report(&TokenBalanceReport {
        address: "0x1111111111111111111111111111111111111111".to_string(),
        chain: "base".to_string(),
        native_symbol: "ETH".to_string(),
        rpc_url: "https://beam.example/base".to_string(),
        tokens: vec![TokenBalanceEntry {
            balance: "1".to_string(),
            decimals: 6,
            is_native: false,
            label: label.to_string(),
            token_address: Some("0x0000000000000000000000000000000000000bee".to_string()),
            value: "1000000".to_string(),
        }],
    });

    assert_eq!(output.compact.as_deref(), Some("BEAM USD|?[31m 1"));
    assert!(output.default.contains("BEAM USD|?[31m"));
    assert!(!output.default.contains('\x1b'));
    assert_eq!(output.value["tokens"][0]["token"], json!(label));
}

#[test]
fn erc20_balance_output_sanitizes_plain_token_labels() {
    let output = render_balance_output(
        "base",
        "BEAM\nUSD\x1b[31m",
        TOKEN_ADDRESS,
        "0x740747e7e3a1e112",
        "12.5",
        6,
        "12500000",
    );

    assert_eq!(
        output.default,
        "12.5 BEAM USD?[31m\nAddress: 0x740747e7e3a1e112\nToken: 0x0000000000000000000000000000000000000bee"
    );
    assert_eq!(output.value["token"], json!("BEAM\nUSD\x1b[31m"));
}

#[tokio::test]
async fn lookup_token_label_sanitizes_on_chain_metadata() {
    let (rpc_url, server) = spawn_token_label_rpc_server("BEAM\nUSD\x1b[31m").await;
    let client = Client::try_new(&rpc_url, None).expect("create token metadata client");

    let label = lookup_token_label(
        &client,
        parse_address(TOKEN_ADDRESS).expect("parse token address"),
    )
    .await
    .expect("lookup token label");
    server.abort();

    assert_eq!(label, "BEAM USD?[31m");
}

async fn spawn_token_label_rpc_server(symbol: &str) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind token label rpc listener");
    let address = listener.local_addr().expect("listener address");
    let symbol = symbol.to_string();

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            serve_token_label_connection(stream, &symbol).await;
        }
    });

    (format!("http://{address}"), server)
}

async fn serve_token_label_connection(mut stream: TcpStream, symbol: &str) {
    let request = read_rpc_request(&mut stream).await;
    let body = token_label_response(&request, symbol);
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .await
        .expect("write token label response");
}

fn token_label_response(request: &Value, symbol: &str) -> String {
    assert_eq!(request["method"], Value::String("eth_call".to_string()));
    assert_eq!(
        &request["params"][0]["data"].as_str().expect("call data")[2..10],
        SYMBOL_SELECTOR
    );

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": encode_string(symbol),
    })
    .to_string()
}

fn encode_string(value: &str) -> String {
    format!(
        "0x{}",
        hex::encode(encode(&[Token::String(value.to_string())]))
    )
}
