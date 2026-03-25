// lint-long-file-override allow-max-lines=300
use std::{
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
    signing::keccak256,
    types::{Bytes, H256, Transaction, TransactionParameters, TransactionReceipt, U64},
};

use super::fixtures::read_rpc_request;
use crate::{
    evm::TransactionOutcome,
    signer::KeySigner,
    transaction::{TransactionExecution, TransactionStatusUpdate, submit_and_wait},
};

#[tokio::test]
async fn submit_and_wait_uses_local_hash_after_duplicate_submission_retry() {
    let (rpc_url, state, server) = spawn_duplicate_submission_rpc_server().await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");
    let updates = Arc::new(Mutex::new(Vec::new()));

    let execution = submit_and_wait(
        &client,
        &signer,
        TransactionParameters {
            chain_id: Some(1),
            gas: U256::from(21_000u64),
            gas_price: Some(U256::from(1_000_000_000u64)),
            nonce: Some(U256::zero()),
            to: Some(Address::from_low_u64_be(0xbeef)),
            value: U256::from(123u64),
            ..Default::default()
        },
        {
            let updates = Arc::clone(&updates);
            move |update| updates.lock().expect("status updates").push(update)
        },
        pending::<()>(),
    )
    .await
    .expect("submit and wait");
    server.abort();

    let (calls, local_tx_hash, receipt_lookup_hashes, transaction_lookup_hashes) = {
        let state = state.lock().expect("rpc state");
        (
            state.calls.clone(),
            state.local_tx_hash.expect("local tx hash"),
            state.receipt_lookup_hashes.clone(),
            state.transaction_lookup_hashes.clone(),
        )
    };
    let tx_hash = format!("{local_tx_hash:#x}");

    assert_eq!(
        execution,
        TransactionExecution::Confirmed(TransactionOutcome {
            block_number: Some(42),
            status: Some(1),
            tx_hash: tx_hash.clone(),
        }),
    );
    assert_eq!(
        updates.lock().expect("status updates").clone(),
        vec![TransactionStatusUpdate::Submitted {
            tx_hash: tx_hash.clone(),
        }],
    );
    assert_eq!(receipt_lookup_hashes, vec![local_tx_hash]);
    assert_eq!(transaction_lookup_hashes, vec![local_tx_hash]);
    assert_eq!(
        rpc_methods(&calls),
        vec![
            "eth_sendRawTransaction",
            "eth_sendRawTransaction",
            "eth_getTransactionByHash",
            "eth_getTransactionReceipt",
        ],
    );
}

#[derive(Default)]
struct DuplicateSubmissionRpcState {
    calls: Vec<Value>,
    send_attempts: usize,
    local_tx_hash: Option<H256>,
    receipt_lookup_hashes: Vec<H256>,
    transaction_lookup_hashes: Vec<H256>,
}

async fn spawn_duplicate_submission_rpc_server() -> (
    String,
    Arc<Mutex<DuplicateSubmissionRpcState>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind duplicate submission rpc listener");
    let address = listener.local_addr().expect("listener address");
    let state = Arc::new(Mutex::new(DuplicateSubmissionRpcState::default()));
    let server_state = Arc::clone(&state);

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_duplicate_submission_rpc_connection(stream, Arc::clone(&server_state)).await;
        }
    });

    (format!("http://{address}"), state, server)
}

async fn handle_duplicate_submission_rpc_connection(
    mut stream: TcpStream,
    state: Arc<Mutex<DuplicateSubmissionRpcState>>,
) {
    let request = read_rpc_request(&mut stream).await;
    let method = request["method"].as_str().expect("rpc method");

    match method {
        "eth_sendRawTransaction" => {
            let attempt = {
                let mut state = state.lock().expect("rpc state");
                state.calls.push(request.clone());
                state.send_attempts += 1;
                state.local_tx_hash = Some(raw_transaction_hash(
                    request["params"][0].as_str().expect("raw transaction"),
                ));
                state.send_attempts
            };

            if attempt == 1 {
                return;
            }

            let body = json!({
                "jsonrpc": "2.0",
                "id": request["id"].clone(),
                "error": {
                    "code": -32000,
                    "message": "already known",
                },
            })
            .to_string();
            write_rpc_response(&mut stream, body).await;
        }
        "eth_getTransactionReceipt" => {
            let body = {
                let mut state = state.lock().expect("rpc state");
                state.calls.push(request.clone());
                let tx_hash = request["params"][0]
                    .as_str()
                    .expect("tx hash")
                    .parse::<H256>()
                    .expect("parse tx hash");
                state.receipt_lookup_hashes.push(tx_hash);

                json!({
                    "jsonrpc": "2.0",
                    "id": request["id"].clone(),
                    "result": successful_receipt(
                        state.local_tx_hash.expect("local tx hash"),
                    ),
                })
                .to_string()
            };
            write_rpc_response(&mut stream, body).await;
        }
        "eth_getTransactionByHash" => {
            let body = {
                let mut state = state.lock().expect("rpc state");
                state.calls.push(request.clone());
                let tx_hash = request["params"][0]
                    .as_str()
                    .expect("tx hash")
                    .parse::<H256>()
                    .expect("parse tx hash");
                state.transaction_lookup_hashes.push(tx_hash);

                json!({
                    "jsonrpc": "2.0",
                    "id": request["id"].clone(),
                    "result": pending_transaction(tx_hash),
                })
                .to_string()
            };
            write_rpc_response(&mut stream, body).await;
        }
        other => panic!("unexpected rpc method {other}"),
    }
}

fn rpc_methods(calls: &[Value]) -> Vec<&str> {
    calls
        .iter()
        .map(|call| call["method"].as_str().expect("rpc method"))
        .collect()
}

fn successful_receipt(tx_hash: H256) -> TransactionReceipt {
    TransactionReceipt {
        block_number: Some(U64::from(42)),
        status: Some(U64::from(1)),
        transaction_hash: tx_hash,
        ..Default::default()
    }
}

fn pending_transaction(tx_hash: H256) -> Transaction {
    Transaction {
        block_number: None,
        from: Some(Address::from_low_u64_be(1)),
        gas: U256::from(30_000u64),
        gas_price: Some(U256::from(1_000_000_000u64)),
        hash: tx_hash,
        input: Bytes::default(),
        nonce: U256::zero(),
        to: Some(Address::from_low_u64_be(0xbeef)),
        value: U256::from(123u64),
        ..Default::default()
    }
}

fn raw_transaction_hash(raw_transaction: &str) -> H256 {
    H256::from_slice(&keccak256(
        &hex::decode(raw_transaction.trim_start_matches("0x")).expect("decode raw transaction"),
    ))
}

async fn write_rpc_response(stream: &mut TcpStream, body: String) {
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
