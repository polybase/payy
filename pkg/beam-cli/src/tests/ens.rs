// lint-long-file-override allow-max-lines=300
use std::sync::{Arc, Mutex};

use contracts::Client;
use serde_json::{Value, json};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use web3::{
    ethabi::{Token, encode},
    signing::namehash,
};

use super::fixtures::{read_rpc_request, spawn_chain_id_rpc_server, test_app};
use crate::{
    config::ChainRpcConfig,
    ens::{import_wallet_name, lookup_verified_ens_name},
    keystore::{KeyStore, StoredKdf, StoredWallet},
    runtime::{BeamApp, InvocationOverrides, parse_address},
};

const ALICE_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
const BOB_ADDRESS: &str = "0x2222222222222222222222222222222222222222";
const ENS_REGISTRY_ADDRESS: &str = "0x00000000000c2e074ec69a0dfb2997ba6c7d2e1e";
const PUBLIC_RESOLVER_ADDRESS: &str = "0x0000000000000000000000000000000000001234";
const REVERSE_RESOLVER_ADDRESS: &str = "0x0000000000000000000000000000000000005678";
const RESOLVER_SELECTOR: &str = "0178b8bf";
const NAME_SELECTOR: &str = "691f3431";
const ADDR_SELECTOR: &str = "3b3b57de";
const SUPPORTS_INTERFACE_SELECTOR: &str = "01ffc9a7";

#[tokio::test]
async fn import_wallet_name_uses_verified_ens_name() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;

    let keystore = app.keystore_store.get().await;
    let name = import_wallet_name(
        &app,
        &keystore,
        None,
        parse_address(ALICE_ADDRESS).expect("parse alice address"),
    )
    .await
    .expect("resolve wallet name");
    server.abort();

    assert_eq!(name, "alice.eth");
}

#[tokio::test]
async fn import_wallet_name_falls_back_when_ens_name_conflicts() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;
    seed_wallets(&app, &[("alice.eth", BOB_ADDRESS)]).await;

    let keystore = app.keystore_store.get().await;
    let name = import_wallet_name(
        &app,
        &keystore,
        None,
        parse_address(ALICE_ADDRESS).expect("parse alice address"),
    )
    .await
    .expect("fall back to generated wallet name");
    server.abort();

    assert_eq!(name, "wallet-1");
}

#[tokio::test]
async fn import_wallet_name_falls_back_when_ethereum_rpc_is_not_mainnet() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(8453).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;

    let keystore = app.keystore_store.get().await;
    let name = import_wallet_name(
        &app,
        &keystore,
        None,
        parse_address(ALICE_ADDRESS).expect("parse alice address"),
    )
    .await
    .expect("fall back to generated wallet name");
    server.abort();

    assert_eq!(name, "wallet-1");
}

#[tokio::test]
async fn lookup_verified_ens_name_rejects_mismatched_forward_record() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", BOB_ADDRESS).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");

    let name = lookup_verified_ens_name(
        &client,
        parse_address(ALICE_ADDRESS).expect("parse alice address"),
    )
    .await
    .expect("lookup ens name");
    server.abort();

    assert_eq!(name, None);
}

#[tokio::test]
async fn lookup_verified_ens_name_rejects_non_mainnet_client() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(8453).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");

    let err = lookup_verified_ens_name(
        &client,
        parse_address(ALICE_ADDRESS).expect("parse alice address"),
    )
    .await
    .expect_err("reject non-mainnet ens client");
    server.abort();

    assert!(matches!(
        err,
        crate::error::Error::RpcChainIdMismatch {
            actual: 8453,
            chain,
            expected: 1,
        } if chain == "ethereum"
    ));
}

pub(super) async fn set_ethereum_rpc(app: &BeamApp, rpc_url: &str) {
    let rpc_url = rpc_url.to_string();
    app.config_store
        .update(move |config| {
            config.rpc_configs.insert(
                "ethereum".to_string(),
                ChainRpcConfig {
                    default_rpc: rpc_url.clone(),
                    rpc_urls: vec![rpc_url.clone()],
                },
            );
        })
        .await
        .expect("persist ethereum rpc");
}

async fn seed_wallets(app: &BeamApp, wallets: &[(&str, &str)]) {
    app.keystore_store
        .set(KeyStore {
            wallets: wallets
                .iter()
                .map(|(name, address)| StoredWallet {
                    address: (*address).to_string(),
                    encrypted_key: "encrypted-key".to_string(),
                    name: (*name).to_string(),
                    salt: "salt".to_string(),
                    kdf: StoredKdf::default(),
                })
                .collect(),
        })
        .await
        .expect("persist keystore");
}

pub(super) async fn spawn_ens_rpc_server(
    ens_name: &str,
    resolved_address: &str,
) -> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    spawn_ens_rpc_server_with_chain_id(1, ens_name, resolved_address).await
}

pub(super) async fn spawn_ens_rpc_server_with_chain_id(
    chain_id: u64,
    ens_name: &str,
    resolved_address: &str,
) -> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind rpc listener");
    let address = listener.local_addr().expect("listener address");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = Arc::clone(&calls);
    let ens_name = ens_name.to_string();
    let resolved_address = resolved_address.to_string();

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            handle_rpc_connection(
                stream,
                Arc::clone(&server_calls),
                chain_id,
                ens_name.clone(),
                resolved_address.clone(),
            )
            .await;
        }
    });

    (format!("http://{address}"), calls, server)
}

async fn handle_rpc_connection(
    mut stream: TcpStream,
    calls: Arc<Mutex<Vec<Value>>>,
    chain_id: u64,
    ens_name: String,
    resolved_address: String,
) {
    let request = read_rpc_request(&mut stream).await;
    calls
        .lock()
        .expect("record rpc request")
        .push(request.clone());

    let body = rpc_response(&request, chain_id, &ens_name, &resolved_address);
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

fn rpc_response(request: &Value, chain_id: u64, ens_name: &str, resolved_address: &str) -> String {
    let result = match request["method"].as_str().expect("rpc method") {
        "eth_chainId" => Value::String(format!("0x{chain_id:x}")),
        "eth_call" => ens_call_result(request, ens_name, resolved_address),
        method => panic!("unexpected ens rpc method: {method}"),
    };

    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    })
    .to_string()
}

fn ens_call_result(request: &Value, ens_name: &str, resolved_address: &str) -> Value {
    let call = &request["params"][0];
    let to = call["to"]
        .as_str()
        .expect("eth_call target")
        .to_ascii_lowercase();
    let data = call["data"]
        .as_str()
        .expect("eth_call data")
        .trim_start_matches("0x");
    let reverse_node = format!(
        "0x{}",
        hex::encode(namehash(&format!(
            "{}.addr.reverse",
            ALICE_ADDRESS.trim_start_matches("0x").to_ascii_lowercase(),
        )))
    );
    let name_node = format!("0x{}", hex::encode(namehash(ens_name)));

    match (to.as_str(), &data[..8]) {
        (ENS_REGISTRY_ADDRESS, RESOLVER_SELECTOR) if data.ends_with(&reverse_node[2..]) => {
            encode_address(REVERSE_RESOLVER_ADDRESS)
        }
        (ENS_REGISTRY_ADDRESS, RESOLVER_SELECTOR) if data.ends_with(&name_node[2..]) => {
            encode_address(PUBLIC_RESOLVER_ADDRESS)
        }
        (REVERSE_RESOLVER_ADDRESS, NAME_SELECTOR) => encode_string(ens_name),
        (PUBLIC_RESOLVER_ADDRESS, SUPPORTS_INTERFACE_SELECTOR) => encode_bool(true),
        (PUBLIC_RESOLVER_ADDRESS, ADDR_SELECTOR) => encode_address(resolved_address),
        _ => panic!("unexpected ens eth_call: to={to} data={data}"),
    }
}

fn encode_address(address: &str) -> Value {
    let address = parse_address(address).expect("parse encoded address");
    Value::String(format!(
        "0x{}",
        hex::encode(encode(&[Token::Address(address)]))
    ))
}

fn encode_bool(value: bool) -> Value {
    Value::String(format!("0x{}", hex::encode(encode(&[Token::Bool(value)]))))
}

fn encode_string(value: &str) -> Value {
    Value::String(format!(
        "0x{}",
        hex::encode(encode(&[Token::String(value.to_string())]))
    ))
}
