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
    ethabi::StateMutability,
    types::{H256, TransactionReceipt, U64},
};

use super::fixtures::read_rpc_request;
use crate::{
    abi::parse_function,
    evm::{FunctionCall, TransactionGas, send_function_with_gas, send_native_with_gas},
    signer::KeySigner,
    transaction::TransactionExecution,
};

#[tokio::test]
async fn native_transfers_with_prepared_gas_skip_reestimation() {
    let (rpc_url, calls, server) = spawn_prepared_gas_rpc_server().await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");

    let outcome = send_native_with_gas(
        &client,
        &signer,
        Address::from_low_u64_be(0xbeef),
        U256::from(123u64),
        Some(prepared_gas()),
        |_| {},
        pending::<()>(),
    )
    .await
    .expect("send native transfer");
    server.abort();

    assert!(
        matches!(outcome, TransactionExecution::Confirmed(ref outcome) if outcome.status == Some(1))
    );
    assert_eq!(
        rpc_methods(&calls.lock().expect("rpc calls")),
        vec![
            "eth_getTransactionCount",
            "eth_chainId",
            "eth_sendRawTransaction",
            "eth_getTransactionReceipt",
        ],
    );
}

#[tokio::test]
async fn function_calls_with_prepared_gas_skip_reestimation() {
    let (rpc_url, calls, server) = spawn_prepared_gas_rpc_server().await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");
    let function = parse_function("transfer(address,uint256)", StateMutability::NonPayable)
        .expect("parse transfer function");
    let args = vec![
        format!("{:#x}", Address::from_low_u64_be(0xbeef)),
        U256::from(123u64).to_string(),
    ];

    let outcome = send_function_with_gas(
        &client,
        &signer,
        FunctionCall {
            args: &args,
            contract: Address::from_low_u64_be(0xfeed),
            function: &function,
            value: U256::zero(),
        },
        Some(prepared_gas()),
        |_| {},
        pending::<()>(),
    )
    .await
    .expect("send function call");
    server.abort();

    assert!(
        matches!(outcome, TransactionExecution::Confirmed(ref outcome) if outcome.status == Some(1))
    );
    assert_eq!(
        rpc_methods(&calls.lock().expect("rpc calls")),
        vec![
            "eth_getTransactionCount",
            "eth_chainId",
            "eth_sendRawTransaction",
            "eth_getTransactionReceipt",
        ],
    );
}

fn prepared_gas() -> TransactionGas {
    TransactionGas {
        gas_limit: U256::from(36_000u64),
        gas_price: U256::from(1_000_000_000u64),
    }
}

async fn spawn_prepared_gas_rpc_server()
-> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind prepared gas rpc listener");
    let address = listener.local_addr().expect("listener address");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = Arc::clone(&calls);

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_prepared_gas_rpc_connection(stream, Arc::clone(&server_calls)).await;
        }
    });

    (format!("http://{address}"), calls, server)
}

async fn handle_prepared_gas_rpc_connection(mut stream: TcpStream, calls: Arc<Mutex<Vec<Value>>>) {
    let request = read_rpc_request(&mut stream).await;
    calls
        .lock()
        .expect("record rpc request")
        .push(request.clone());

    let body = rpc_response(&request);
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

fn rpc_response(request: &Value) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_getTransactionCount" => serde_json::to_value(U256::zero()).expect("nonce"),
        "eth_chainId" => serde_json::to_value(U256::one()).expect("chain id"),
        "eth_sendRawTransaction" => serde_json::to_value(H256::from_low_u64_be(7)).expect("hash"),
        "eth_getTransactionReceipt" => serde_json::to_value(successful_receipt()).expect("receipt"),
        other => panic!("unexpected rpc method {other}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}

fn successful_receipt() -> TransactionReceipt {
    TransactionReceipt {
        block_number: Some(U64::from(42)),
        status: Some(U64::from(1)),
        transaction_hash: H256::from_low_u64_be(7),
        ..Default::default()
    }
}
