use std::sync::{Arc, Mutex};

use contracts::{Address, Client, U256};
use serde_json::{Value, json};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};

use super::fixtures::read_rpc_request;
use crate::evm::estimate_native_gas;

#[tokio::test]
async fn eip1559_fee_estimation_uses_priority_floor_for_weak_rewards() {
    let (rpc_url, calls, server) = spawn_fee_rpc_server(FeeScenario::WeakReward).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");

    let gas = estimate_native_gas(
        &client,
        Address::from_low_u64_be(1),
        Address::from_low_u64_be(2),
        U256::zero(),
    )
    .await
    .expect("estimate gas");
    server.abort();

    assert_eq!(gas.gas_limit, U256::from(36_000u64));
    assert_eq!(gas.gas_price_for_display(), U256::from(3_000_000_000u64));
    assert_eq!(
        rpc_methods(&calls.lock().expect("rpc calls")),
        vec!["eth_estimateGas", "eth_chainId", "eth_feeHistory"]
    );
}

#[tokio::test]
async fn eip1559_fee_estimation_uses_floor_when_rewards_are_missing() {
    let (rpc_url, _calls, server) = spawn_fee_rpc_server(FeeScenario::MissingReward).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");

    let gas = estimate_native_gas(
        &client,
        Address::from_low_u64_be(1),
        Address::from_low_u64_be(2),
        U256::zero(),
    )
    .await
    .expect("estimate gas");
    server.abort();

    assert_eq!(gas.gas_price_for_display(), U256::from(2_001_000_000u64));
}

#[tokio::test]
async fn fee_estimation_falls_back_to_legacy_when_fee_history_is_missing() {
    let (rpc_url, calls, server) = spawn_fee_rpc_server(FeeScenario::NoFeeHistory).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");

    let gas = estimate_native_gas(
        &client,
        Address::from_low_u64_be(1),
        Address::from_low_u64_be(2),
        U256::zero(),
    )
    .await
    .expect("estimate gas");
    server.abort();

    assert_eq!(gas.gas_price_for_display(), U256::from(1_100_000_000u64));
    assert_eq!(
        rpc_methods(&calls.lock().expect("rpc calls")),
        vec![
            "eth_estimateGas",
            "eth_chainId",
            "eth_feeHistory",
            "eth_gasPrice",
        ]
    );
}

#[derive(Clone, Copy)]
enum FeeScenario {
    WeakReward,
    MissingReward,
    NoFeeHistory,
}

async fn spawn_fee_rpc_server(
    scenario: FeeScenario,
) -> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fee rpc listener");
    let address = listener.local_addr().expect("listener address");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = Arc::clone(&calls);

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_fee_rpc_connection(stream, Arc::clone(&server_calls), scenario).await;
        }
    });

    (format!("http://{address}"), calls, server)
}

async fn handle_fee_rpc_connection(
    mut stream: TcpStream,
    calls: Arc<Mutex<Vec<Value>>>,
    scenario: FeeScenario,
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

fn rpc_response(request: &Value, scenario: FeeScenario) -> String {
    if request["method"] == "eth_feeHistory" && matches!(scenario, FeeScenario::NoFeeHistory) {
        return json!({
            "jsonrpc": "2.0",
            "id": request["id"].clone(),
            "error": {
                "code": -32601,
                "message": "method not found",
            },
        })
        .to_string();
    }

    let result = match request["method"].as_str().expect("rpc method") {
        "eth_estimateGas" => serde_json::to_value(U256::from(30_000u64)).expect("estimate gas"),
        "eth_chainId" => serde_json::to_value(chain_id(scenario)).expect("chain id"),
        "eth_feeHistory" => fee_history(scenario),
        "eth_gasPrice" => serde_json::to_value(U256::from(1_000_000_000u64)).expect("gas price"),
        other => panic!("unexpected rpc method {other}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}

fn fee_history(scenario: FeeScenario) -> Value {
    match scenario {
        FeeScenario::WeakReward => json!({
            "oldestBlock": "0x1",
            "baseFeePerGas": ["0x3b9aca00", "0x3b9aca00"],
            "gasUsedRatio": [0.5],
            "reward": [["0x1"]],
        }),
        FeeScenario::MissingReward => json!({
            "oldestBlock": "0x1",
            "baseFeePerGas": ["0x3b9aca00", "0x3b9aca00"],
            "gasUsedRatio": [0.5],
        }),
        FeeScenario::NoFeeHistory => unreachable!("handled before result response"),
    }
}

fn chain_id(scenario: FeeScenario) -> U256 {
    match scenario {
        FeeScenario::WeakReward | FeeScenario::NoFeeHistory => U256::one(),
        FeeScenario::MissingReward => U256::from(8453u64),
    }
}

fn rpc_methods(calls: &[Value]) -> Vec<&str> {
    calls
        .iter()
        .map(|call| call["method"].as_str().expect("rpc method"))
        .collect()
}
