// lint-long-file-override allow-max-lines=300
use std::sync::{Arc, Mutex};

use contracts::{Address, U256};
use serde_json::{Value, json};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use web3::types::{H256, TransactionReceipt, U64};

use super::execution::execute_plan_with_signer;
use crate::{
    apps::model::{ActionPlan, ActionStep, ApprovalFeeCap},
    error::Error,
    output::OutputMode,
    runtime::InvocationOverrides,
    signer::KeySigner,
    tests::fixtures::{read_rpc_request, test_app_with_output},
};

const BASE_CHAIN_ID: u64 = 8453;
const MAX_FEE_PER_GAS: u64 = 4_000_000_000;
const MAX_PRIORITY_FEE_PER_GAS: u64 = 2_000_000_000;
const PADDED_GAS_LIMIT: u64 = 25_200;

#[tokio::test]
async fn app_transaction_with_low_gas_price_is_repriced_as_eip1559() {
    let (rpc_url, state, server) = spawn_app_execution_rpc_server().await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            rpc: Some(rpc_url),
            ..InvocationOverrides::default()
        },
    )
    .await;
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");
    let output = execute_plan_with_signer(&app, &action_plan(), &fee_caps(), &signer)
        .await
        .expect("execute app plan");
    server.abort();

    let raw_transaction = state
        .lock()
        .expect("rpc state")
        .raw_transaction
        .clone()
        .expect("raw transaction");
    let signed = decode_typed_transaction(&raw_transaction);

    assert_eq!(signed.transaction_type, 2);
    assert_eq!(signed.max_priority_fee_per_gas, MAX_PRIORITY_FEE_PER_GAS);
    assert_eq!(signed.max_fee_per_gas, MAX_FEE_PER_GAS);
    assert_eq!(output.value["steps"][0]["fee"]["fee_mode"], "eip1559");
    assert_eq!(
        output.value["steps"][0]["fee"]["max_fee_per_gas"],
        MAX_FEE_PER_GAS.to_string()
    );
    assert_eq!(
        output.value["steps"][0]["fee"]["max_network_fee_wei"],
        (PADDED_GAS_LIMIT * MAX_FEE_PER_GAS).to_string()
    );
    assert_eq!(output.value["steps"][0]["fee"].get("gas_price"), None);
}

#[tokio::test]
async fn app_transaction_without_fee_cap_fails_closed() {
    let (rpc_url, _state, server) = spawn_app_execution_rpc_server().await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            rpc: Some(rpc_url),
            ..InvocationOverrides::default()
        },
    )
    .await;
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");

    let error = execute_plan_with_signer(&app, &action_plan(), &[], &signer)
        .await
        .expect_err("reject missing fee cap");
    server.abort();

    assert!(matches!(
        error,
        Error::App(crate::apps::Error::ApprovalFeeCapMissing { step_index: 0 })
    ));
}

#[tokio::test]
async fn app_transaction_over_fee_cap_fails_before_submission() {
    let (rpc_url, state, server) = spawn_app_execution_rpc_server().await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            rpc: Some(rpc_url),
            ..InvocationOverrides::default()
        },
    )
    .await;
    let signer = KeySigner::from_slice(&[7u8; 32]).expect("create signer");
    let mut fee_caps = fee_caps();
    fee_caps[0].approved_max_total_fee_wei = "1".to_string();

    let error = execute_plan_with_signer(&app, &action_plan(), &fee_caps, &signer)
        .await
        .expect_err("reject fee cap breach");
    server.abort();

    assert!(matches!(error, Error::TransactionFeeCapExceeded { .. }));
    assert!(state.lock().expect("rpc state").raw_transaction.is_none());
}

#[derive(Default)]
struct AppExecutionRpcState {
    raw_transaction: Option<String>,
}

struct DecodedTypedTransaction {
    transaction_type: u8,
    max_priority_fee_per_gas: u64,
    max_fee_per_gas: u64,
}

async fn spawn_app_execution_rpc_server() -> (
    String,
    Arc<Mutex<AppExecutionRpcState>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind app execution rpc listener");
    let address = listener.local_addr().expect("listener address");
    let state = Arc::new(Mutex::new(AppExecutionRpcState::default()));
    let server_state = Arc::clone(&state);

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_app_execution_rpc_connection(stream, Arc::clone(&server_state)).await;
        }
    });

    (format!("http://{address}"), state, server)
}

async fn handle_app_execution_rpc_connection(
    mut stream: TcpStream,
    state: Arc<Mutex<AppExecutionRpcState>>,
) {
    let request = read_rpc_request(&mut stream).await;
    let method = request["method"].as_str().expect("rpc method");
    if method == "eth_sendRawTransaction" {
        state.lock().expect("rpc state").raw_transaction = Some(
            request["params"][0]
                .as_str()
                .expect("raw transaction")
                .to_string(),
        );
    }

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

fn rpc_response(request: &Value) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_chainId" => serde_json::to_value(U256::from(BASE_CHAIN_ID)).expect("chain id"),
        "eth_feeHistory" => json!({
            "oldestBlock": "0x1",
            "baseFeePerGas": ["0x3b9aca00", "0x3b9aca00"],
            "gasUsedRatio": [0.5],
            "reward": [["0x77359400"]],
        }),
        "eth_getTransactionCount" => serde_json::to_value(U256::zero()).expect("nonce"),
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

fn action_plan() -> ActionPlan {
    ActionPlan {
        app_id: "uniswap".to_string(),
        app_version: "1.0.0".to_string(),
        wasm_sha256: "sha256:wasm".to_string(),
        manifest_sha256: "sha256:manifest".to_string(),
        command: "swap USDC ETH 1".to_string(),
        wallet: None,
        chain: "base".to_string(),
        steps: vec![ActionStep {
            kind: "transaction".to_string(),
            summary: "Swap 1 USDC for ETH".to_string(),
            target: Some(format!("{:#x}", Address::from_low_u64_be(0xfeed))),
            selector: Some("0x3593564c".to_string()),
            spender: None,
            value: Some("0".to_string()),
            metadata: json!({
                "transaction": {
                    "data": "0x3593564c",
                    "gas_limit": "21000",
                    "gas_price": "1",
                    "to": format!("{:#x}", Address::from_low_u64_be(0xfeed)),
                    "value": "0",
                },
            }),
        }],
        bindings: Vec::new(),
        constraints: Vec::new(),
        dynamic_contracts: Vec::new(),
        expires_at: 9_999_999_999,
    }
}

fn fee_caps() -> Vec<ApprovalFeeCap> {
    vec![ApprovalFeeCap {
        step_index: 0,
        approved_gas_limit: PADDED_GAS_LIMIT.to_string(),
        approved_max_fee_per_gas: MAX_FEE_PER_GAS.to_string(),
        approved_max_total_fee_wei: "200000000000000".to_string(),
        fee_mode: "eip1559".to_string(),
        approved_max_priority_fee_per_gas: Some(MAX_PRIORITY_FEE_PER_GAS.to_string()),
    }]
}

fn decode_typed_transaction(raw_transaction: &str) -> DecodedTypedTransaction {
    let bytes = hex::decode(raw_transaction.trim_start_matches("0x")).expect("decode transaction");
    assert!(!bytes.is_empty(), "raw transaction should not be empty");
    let rlp = rlp::Rlp::new(&bytes[1..]);
    DecodedTypedTransaction {
        transaction_type: bytes[0],
        max_priority_fee_per_gas: rlp_u64_at(&rlp, 2),
        max_fee_per_gas: rlp_u64_at(&rlp, 3),
    }
}

fn rlp_u64_at(rlp: &rlp::Rlp<'_>, index: usize) -> u64 {
    let data = rlp
        .at(index)
        .expect("decode rlp item")
        .data()
        .expect("decode rlp integer");
    U256::from_big_endian(data).as_u64()
}
