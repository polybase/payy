// lint-long-file-override allow-max-lines=300
use std::{
    collections::HashMap,
    future::pending,
    sync::{Arc, Mutex},
};

use contracts::{Address, Client, U256};
use serde_json::{Value, json};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use web3::{
    ethabi::{StateMutability, Token, encode},
    types::{H256, TransactionReceipt, U64},
};

use super::fixtures::read_rpc_request;
use crate::{
    abi::parse_function,
    evm::{TransactionOutcome, call_function, send_native},
    signer::KeySigner,
    transaction::TransactionExecution,
};

#[tokio::test]
async fn call_function_retries_transient_eth_call_failures() {
    let (rpc_url, state, server) =
        spawn_retry_rpc_server("eth_call", RetryRpcMode::CallReturnsDecimals).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let function = parse_function("decimals():(uint8)", StateMutability::View)
        .expect("parse decimals function");

    let outcome = call_function(
        &client,
        None,
        Address::from_low_u64_be(0xbeef),
        &function,
        &[],
    )
    .await
    .expect("call function");
    server.abort();

    assert_eq!(
        outcome.decoded,
        Some(json!(["6"])),
        "expected decoded eth_call result after retry",
    );
    assert_eq!(method_count(&state, "eth_call"), 2);
}

#[tokio::test]
async fn send_native_retries_transient_estimate_gas_failures() {
    let (rpc_url, state, server) =
        spawn_retry_rpc_server("eth_estimateGas", RetryRpcMode::ConfirmedTransfer).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");

    let outcome = send_native(
        &client,
        &signer,
        Address::from_low_u64_be(0xbeef),
        U256::from(123u64),
        |_| {},
        pending::<()>(),
    )
    .await
    .expect("submit native transfer");
    server.abort();

    assert_eq!(
        outcome,
        TransactionExecution::Confirmed(TransactionOutcome {
            block_number: Some(42),
            status: Some(1),
            tx_hash: format!("{:#x}", H256::from_low_u64_be(7)),
        }),
    );
    assert_eq!(method_count(&state, "eth_estimateGas"), 2);
    assert_eq!(method_count(&state, "eth_sendRawTransaction"), 1);
    assert_eq!(
        rpc_methods(&state)[..6],
        [
            "eth_estimateGas",
            "eth_estimateGas",
            "eth_gasPrice",
            "eth_getTransactionCount",
            "eth_chainId",
            "eth_sendRawTransaction",
        ],
    );
}

#[tokio::test]
async fn send_native_retries_transient_raw_submission_failures() {
    let (rpc_url, state, server) =
        spawn_retry_rpc_server("eth_sendRawTransaction", RetryRpcMode::ConfirmedTransfer).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");

    let outcome = send_native(
        &client,
        &signer,
        Address::from_low_u64_be(0xbeef),
        U256::from(123u64),
        |_| {},
        pending::<()>(),
    )
    .await
    .expect("submit native transfer");
    server.abort();

    assert_eq!(
        outcome,
        TransactionExecution::Confirmed(TransactionOutcome {
            block_number: Some(42),
            status: Some(1),
            tx_hash: format!("{:#x}", H256::from_low_u64_be(7)),
        }),
    );
    assert_eq!(method_count(&state, "eth_estimateGas"), 1);
    assert_eq!(method_count(&state, "eth_sendRawTransaction"), 2);
    assert_eq!(
        rpc_methods(&state)[..7],
        [
            "eth_estimateGas",
            "eth_gasPrice",
            "eth_getTransactionCount",
            "eth_chainId",
            "eth_sendRawTransaction",
            "eth_sendRawTransaction",
            "eth_getTransactionReceipt",
        ],
    );
}

#[derive(Clone, Copy)]
enum RetryRpcMode {
    CallReturnsDecimals,
    ConfirmedTransfer,
}

#[derive(Default)]
struct RetryRpcState {
    calls: Vec<Value>,
    failures_remaining: HashMap<String, usize>,
}

async fn spawn_retry_rpc_server(
    fail_once_for: &str,
    mode: RetryRpcMode,
) -> (
    String,
    Arc<Mutex<RetryRpcState>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind retry rpc listener");
    let address = listener.local_addr().expect("listener address");
    let state = Arc::new(Mutex::new(RetryRpcState {
        failures_remaining: HashMap::from([(fail_once_for.to_string(), 1)]),
        ..Default::default()
    }));
    let server_state = Arc::clone(&state);

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_retry_rpc_connection(stream, Arc::clone(&server_state), mode).await;
        }
    });

    (format!("http://{address}"), state, server)
}

async fn handle_retry_rpc_connection(
    mut stream: TcpStream,
    state: Arc<Mutex<RetryRpcState>>,
    mode: RetryRpcMode,
) {
    let request = read_rpc_request(&mut stream).await;
    let method = request["method"].as_str().expect("rpc method").to_string();
    let should_drop = {
        let mut state = state.lock().expect("rpc state");
        state.calls.push(request.clone());
        match state.failures_remaining.get_mut(&method) {
            Some(remaining) if *remaining > 0 => {
                *remaining -= 1;
                true
            }
            _ => false,
        }
    };

    if should_drop {
        return;
    }

    let body = rpc_response(&request, mode);
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

fn rpc_response(request: &Value, mode: RetryRpcMode) -> String {
    let method = request["method"].as_str().expect("rpc method");
    let result = match (mode, method) {
        (RetryRpcMode::CallReturnsDecimals, "eth_call") => encode_uint(6),
        (RetryRpcMode::ConfirmedTransfer, "eth_estimateGas") => {
            serde_json::to_value(U256::from(30_000u64)).expect("estimate gas")
        }
        (RetryRpcMode::ConfirmedTransfer, "eth_gasPrice") => {
            serde_json::to_value(U256::from(1_000_000_000u64)).expect("gas price")
        }
        (RetryRpcMode::ConfirmedTransfer, "eth_getTransactionCount") => {
            serde_json::to_value(U256::zero()).expect("nonce")
        }
        (RetryRpcMode::ConfirmedTransfer, "eth_chainId") => {
            serde_json::to_value(U256::one()).expect("chain id")
        }
        (RetryRpcMode::ConfirmedTransfer, "eth_sendRawTransaction") => {
            serde_json::to_value(H256::from_low_u64_be(7)).expect("tx hash")
        }
        (RetryRpcMode::ConfirmedTransfer, "eth_getTransactionReceipt") => {
            serde_json::to_value(successful_receipt()).expect("receipt")
        }
        _ => panic!("unexpected retry rpc method: {method}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}

fn encode_uint(value: u64) -> Value {
    Value::String(format!(
        "0x{}",
        hex::encode(encode(&[Token::Uint(U256::from(value))])),
    ))
}

fn successful_receipt() -> TransactionReceipt {
    TransactionReceipt {
        block_number: Some(U64::from(42)),
        status: Some(U64::from(1)),
        transaction_hash: H256::from_low_u64_be(7),
        ..Default::default()
    }
}

fn method_count(state: &Arc<Mutex<RetryRpcState>>, method: &str) -> usize {
    state
        .lock()
        .expect("rpc state")
        .calls
        .iter()
        .filter(|call| call["method"] == Value::String(method.to_string()))
        .count()
}

fn rpc_methods(state: &Arc<Mutex<RetryRpcState>>) -> Vec<String> {
    state
        .lock()
        .expect("rpc state")
        .calls
        .iter()
        .map(|call| call["method"].as_str().expect("rpc method").to_string())
        .collect()
}
