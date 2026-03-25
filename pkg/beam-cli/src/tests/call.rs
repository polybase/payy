use super::{
    ens::{set_ethereum_rpc, spawn_ens_rpc_server},
    fixtures::test_app,
};
use contracts::U256;
use web3::ethabi::StateMutability;

use crate::{
    abi::parse_function,
    commands::call::{parse_transaction_value, resolve_address_args},
    keystore::{KeyStore, StoredKdf, StoredWallet},
    runtime::{BeamApp, InvocationOverrides},
};

const ALICE_ADDRESS: &str = "0x1111111111111111111111111111111111111111";

async fn seed_wallet(app: &BeamApp) {
    app.keystore_store
        .set(KeyStore {
            wallets: vec![StoredWallet {
                address: ALICE_ADDRESS.to_string(),
                encrypted_key: "encrypted-key".to_string(),
                name: "alice".to_string(),
                salt: "salt".to_string(),
                kdf: StoredKdf::default(),
            }],
        })
        .await
        .expect("persist keystore");
}

#[tokio::test]
async fn resolves_wallet_names_for_address_arguments() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_wallet(&app).await;
    let function = parse_function("balanceOf(address):(uint256)", StateMutability::View)
        .expect("parse function");

    let resolved = resolve_address_args(&app, &function, &["alice".to_string()])
        .await
        .expect("resolve address args");

    assert_eq!(resolved, vec![ALICE_ADDRESS.to_string()]);
}

#[tokio::test]
async fn resolves_ens_names_for_address_arguments() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;
    let function = parse_function("balanceOf(address):(uint256)", StateMutability::View)
        .expect("parse function");

    let resolved = resolve_address_args(&app, &function, &["alice.eth".to_string()])
        .await
        .expect("resolve ens address arg");
    server.abort();

    assert_eq!(resolved, vec![ALICE_ADDRESS.to_string()]);
}

#[tokio::test]
async fn leaves_non_address_arguments_unchanged() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let function =
        parse_function("transfer(uint256)", StateMutability::NonPayable).expect("parse function");

    let resolved = resolve_address_args(&app, &function, &["5".to_string()])
        .await
        .expect("leave numeric args unchanged");

    assert_eq!(resolved, vec!["5".to_string()]);
}

#[test]
fn send_transaction_value_defaults_to_zero() {
    let value = parse_transaction_value(None).expect("default send value");

    assert_eq!(value, U256::zero());
}

#[test]
fn send_transaction_value_parses_native_units() {
    let value = parse_transaction_value(Some("0.25")).expect("parse send value");

    assert_eq!(
        value,
        U256::from_dec_str("250000000000000000").expect("parse wei value"),
    );
}
