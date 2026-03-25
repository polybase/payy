use std::{
    future::pending,
    sync::{Arc, Mutex},
    time::Duration,
};

use contracts::{Address, Client, U256};
use serde_json::{Value, json};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use web3::types::{Bytes, H256, Transaction};

use super::fixtures::read_rpc_request;
use crate::transaction::{
    DroppedTransaction, TransactionExecution, TransactionStatusUpdate, wait_for_completion,
};

#[tokio::test]
async fn wait_for_completion_returns_dropped_when_transaction_disappears() {
    let (rpc_url, calls, server) = spawn_transaction_wait_rpc_server().await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let updates = Arc::new(Mutex::new(Vec::new()));
    let tx_hash = format!("{:#x}", H256::from_low_u64_be(7));

    let execution = wait_for_completion(
        &client,
        tx_hash.clone(),
        {
            let updates = Arc::clone(&updates);
            move |update| updates.lock().expect("status updates").push(update)
        },
        pending::<()>(),
        Duration::from_millis(5),
        Duration::from_millis(12),
    )
    .await
    .expect("wait for completion");
    server.abort();

    assert_eq!(
        execution,
        TransactionExecution::Dropped(DroppedTransaction {
            block_number: None,
            tx_hash: tx_hash.clone(),
        }),
    );
    assert_eq!(
        updates.lock().expect("status updates").clone(),
        vec![
            TransactionStatusUpdate::Pending {
                tx_hash: tx_hash.clone(),
            },
            TransactionStatusUpdate::Dropped {
                tx_hash: tx_hash.clone(),
            },
        ],
    );

    let calls = calls.lock().expect("rpc calls").calls.clone();
    let methods = rpc_methods(&calls);
    assert!(
        methods.len() >= 4,
        "expected repeated receipt and transaction polls"
    );
    assert_eq!(methods[0], "eth_getTransactionReceipt");
    assert_eq!(methods[1], "eth_getTransactionByHash");
    assert_eq!(methods[2], "eth_getTransactionReceipt");
    assert_eq!(methods[3], "eth_getTransactionByHash");
}

#[derive(Default)]
struct TransactionWaitRpcState {
    calls: Vec<Value>,
    transaction_lookups: usize,
}

async fn spawn_transaction_wait_rpc_server() -> (
    String,
    Arc<Mutex<TransactionWaitRpcState>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind transaction wait rpc listener");
    let address = listener.local_addr().expect("listener address");
    let calls = Arc::new(Mutex::new(TransactionWaitRpcState::default()));
    let server_calls = Arc::clone(&calls);

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_transaction_wait_rpc_connection(stream, Arc::clone(&server_calls)).await;
        }
    });

    (format!("http://{address}"), calls, server)
}

async fn handle_transaction_wait_rpc_connection(
    mut stream: TcpStream,
    calls: Arc<Mutex<TransactionWaitRpcState>>,
) {
    let request = read_rpc_request(&mut stream).await;
    let method = request["method"].as_str().expect("rpc method").to_string();
    let first_transaction_lookup = {
        let mut calls = calls.lock().expect("record rpc request");
        calls.calls.push(request.clone());
        if method == "eth_getTransactionByHash" {
            let first_lookup = calls.transaction_lookups == 0;
            calls.transaction_lookups += 1;
            first_lookup
        } else {
            false
        }
    };

    let body = rpc_response(&request, &method, first_transaction_lookup);
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

fn rpc_methods(calls: &[Value]) -> Vec<&str> {
    calls
        .iter()
        .map(|call| call["method"].as_str().expect("rpc method"))
        .collect()
}

fn rpc_response(request: &Value, method: &str, first_transaction_lookup: bool) -> String {
    let result = match method {
        "eth_getTransactionReceipt" => Value::Null,
        "eth_getTransactionByHash" if first_transaction_lookup => {
            serde_json::to_value(pending_transaction()).expect("pending transaction")
        }
        "eth_getTransactionByHash" => Value::Null,
        other => panic!("unexpected rpc method {other}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}

fn pending_transaction() -> Transaction {
    Transaction {
        block_number: None,
        from: Some(Address::from_low_u64_be(1)),
        gas: U256::from(30_000u64),
        gas_price: Some(U256::from(1_000_000_000u64)),
        hash: H256::from_low_u64_be(7),
        input: Bytes::default(),
        nonce: U256::zero(),
        to: Some(Address::from_low_u64_be(0xbeef)),
        value: U256::from(123u64),
        ..Default::default()
    }
}
