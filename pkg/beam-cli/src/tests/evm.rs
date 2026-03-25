// lint-long-file-override allow-max-lines=300
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
    time::sleep,
};
use web3::types::{Bytes, H256, Transaction, TransactionReceipt, U64};

use super::fixtures::read_rpc_request;
use crate::{
    error::Error,
    evm::{outcome_from_receipt, parse_units, send_native},
    signer::{KeySigner, Signer},
    transaction::{TransactionExecution, TransactionStatusUpdate},
};

#[test]
fn builds_transaction_outcome_for_successful_receipt() {
    let receipt = receipt_with_status(Some(1));

    let outcome = outcome_from_receipt(receipt).expect("build outcome for successful receipt");

    assert_eq!(outcome.block_number, Some(42));
    assert_eq!(outcome.status, Some(1));
    assert_eq!(outcome.tx_hash, format!("{:#x}", H256::from_low_u64_be(7)));
}

#[test]
fn rejects_reverted_receipt() {
    let receipt = receipt_with_status(Some(0));

    let err = outcome_from_receipt(receipt).expect_err("reject reverted receipt");

    assert!(matches!(err, Error::TransactionFailed { status: 0, .. }));
}

#[test]
fn rejects_receipt_without_status() {
    let receipt = receipt_with_status(None);

    let err = outcome_from_receipt(receipt).expect_err("reject missing receipt status");

    assert!(matches!(err, Error::TransactionStatusMissing { .. }));
}

#[test]
fn rejects_amounts_that_overflow_u256() {
    let value = "115792089237316195423570985008687907853269984665640564039457584007913129639936";
    let err = parse_units(value, 0).expect_err("reject overflowing amount");

    assert!(matches!(err, Error::InvalidAmount { value: got } if got == value));
}

#[test]
fn rejects_amounts_with_unsupported_decimals() {
    let err = parse_units("1", 78).expect_err("reject unsupported decimals");

    assert!(matches!(
        err,
        Error::UnsupportedDecimals {
            decimals: 78,
            max: 77,
        }
    ));
}

#[test]
fn rejects_scaled_amounts_that_overflow_u256() {
    let value = "115792089237316195423570985008687907853269984665640564039457584007913129639935";
    let err = parse_units(value, 1).expect_err("reject scaled overflow");

    assert!(matches!(err, Error::InvalidAmount { value: got } if got == value));
}

#[tokio::test]
async fn native_transfers_estimate_gas_before_submission() {
    let (rpc_url, calls, server) = spawn_rpc_server(RpcScenario::Confirmed).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");
    let recipient = Address::from_low_u64_be(0xbeef);
    let amount = U256::from(123u64);

    let outcome = send_native(&client, &signer, recipient, amount, |_| {}, pending::<()>())
        .await
        .expect("send native transfer");
    server.abort();

    assert!(
        matches!(outcome, TransactionExecution::Confirmed(ref outcome) if outcome.status == Some(1))
    );

    let calls = calls.lock().expect("rpc calls").clone();
    let methods = rpc_methods(&calls);
    assert_eq!(
        methods,
        vec![
            "eth_estimateGas",
            "eth_gasPrice",
            "eth_getTransactionCount",
            "eth_chainId",
            "eth_sendRawTransaction",
            "eth_getTransactionReceipt",
        ]
    );

    let estimate = &calls[0]["params"][0];
    assert_eq!(
        estimate["from"],
        Value::String(format!("{:#x}", signer.address()))
    );
    assert_eq!(estimate["to"], Value::String(format!("{recipient:#x}")));
    assert_eq!(estimate["value"], Value::String("0x7b".to_string()));
    assert_eq!(estimate["data"], Value::String("0x".to_string()));
}

#[tokio::test]
async fn native_transfers_return_pending_hash_when_wait_is_cancelled() {
    let (rpc_url, calls, server) = spawn_rpc_server(RpcScenario::Pending).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");
    let recipient = Address::from_low_u64_be(0xbeef);
    let amount = U256::from(123u64);
    let updates = Arc::new(Mutex::new(Vec::new()));

    let outcome = send_native(
        &client,
        &signer,
        recipient,
        amount,
        {
            let updates = Arc::clone(&updates);
            move |update| updates.lock().expect("status updates").push(update)
        },
        sleep(Duration::from_millis(10)),
    )
    .await
    .expect("submit native transfer");
    server.abort();

    let pending = match outcome {
        TransactionExecution::Pending(pending) => pending,
        other => panic!("expected pending transfer, got {other:?}"),
    };
    assert!(pending.block_number.is_none());

    let tx_hash = pending.tx_hash;
    let updates = updates.lock().expect("status updates").clone();
    assert!(updates.contains(&TransactionStatusUpdate::Submitted {
        tx_hash: tx_hash.clone(),
    }));
    if updates.len() == 2 {
        assert_eq!(updates[1], TransactionStatusUpdate::Pending { tx_hash });
    }

    let calls = calls.lock().expect("rpc calls").clone();
    let methods = rpc_methods(&calls);
    assert_eq!(
        &methods[..5],
        &[
            "eth_estimateGas",
            "eth_gasPrice",
            "eth_getTransactionCount",
            "eth_chainId",
            "eth_sendRawTransaction",
        ]
    );
    if methods.len() == 7 {
        assert_eq!(
            &methods[5..],
            &["eth_getTransactionReceipt", "eth_getTransactionByHash"]
        );
    } else {
        assert_eq!(methods.len(), 5);
    }
}

fn receipt_with_status(status: Option<u64>) -> TransactionReceipt {
    TransactionReceipt {
        block_number: Some(U64::from(42)),
        status: status.map(U64::from),
        transaction_hash: H256::from_low_u64_be(7),
        ..Default::default()
    }
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

#[derive(Clone, Copy)]
enum RpcScenario {
    Confirmed,
    Pending,
}

async fn spawn_rpc_server(
    scenario: RpcScenario,
) -> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind rpc listener");
    let address = listener.local_addr().expect("listener address");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = Arc::clone(&calls);

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_rpc_connection(stream, Arc::clone(&server_calls), scenario).await;
        }
    });

    (format!("http://{address}"), calls, server)
}

async fn handle_rpc_connection(
    mut stream: TcpStream,
    calls: Arc<Mutex<Vec<Value>>>,
    scenario: RpcScenario,
) {
    let request = read_rpc_request(&mut stream).await;
    calls
        .lock()
        .expect("record rpc request")
        .push(request.clone());

    let body = rpc_response(&request, scenario);
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

fn rpc_response(request: &Value, scenario: RpcScenario) -> String {
    let method = request["method"].as_str().expect("rpc method");
    let result = match method {
        "eth_estimateGas" => serde_json::to_value(U256::from(30_000u64)).expect("estimate gas"),
        "eth_gasPrice" => serde_json::to_value(U256::from(1_000_000_000u64)).expect("gas price"),
        "eth_getTransactionCount" => serde_json::to_value(U256::zero()).expect("nonce"),
        "eth_chainId" => serde_json::to_value(U256::one()).expect("chain id"),
        "eth_sendRawTransaction" => serde_json::to_value(H256::from_low_u64_be(7)).expect("hash"),
        "eth_getTransactionReceipt" => match scenario {
            RpcScenario::Confirmed => {
                serde_json::to_value(receipt_with_status(Some(1))).expect("receipt")
            }
            RpcScenario::Pending => Value::Null,
        },
        "eth_getTransactionByHash" => match scenario {
            RpcScenario::Confirmed => Value::Null,
            RpcScenario::Pending => {
                serde_json::to_value(pending_transaction()).expect("transaction")
            }
        },
        other => panic!("unexpected rpc method {other}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}
