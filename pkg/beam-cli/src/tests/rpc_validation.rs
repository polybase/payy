use super::{
    ens::set_ethereum_rpc,
    fixtures::{spawn_chain_id_rpc_server, test_app},
};
use crate::{
    commands::{interactive::set_repl_rpc_override, wallet::rename_wallet},
    config::ChainRpcConfig,
    error::Error,
    keystore::{KeyStore, StoredKdf, StoredWallet},
    runtime::{BeamApp, InvocationOverrides},
};

const ALICE_ADDRESS: &str = "0x1111111111111111111111111111111111111111";

async fn set_default_chain(app: &BeamApp, default_chain: &str) {
    let mut config = app.config_store.get().await;
    config.default_chain = default_chain.to_string();
    app.config_store.set(config).await.expect("persist config");
}

async fn set_rpc_config(app: &BeamApp, chain_key: &str, rpc_config: ChainRpcConfig) {
    let mut config = app.config_store.get().await;
    config.rpc_configs.insert(chain_key.to_string(), rpc_config);
    app.config_store
        .set(config)
        .await
        .expect("persist rpc config");
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

fn assert_ethereum_chain_id_mismatch(err: Error) {
    assert!(matches!(
        err,
        Error::RpcChainIdMismatch {
            actual: 8453,
            chain,
            expected: 1,
        } if chain == "ethereum"
    ));
}

#[tokio::test]
async fn active_chain_client_rejects_cli_rpc_override_with_mismatched_chain_id() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(1).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        chain: Some("base".to_string()),
        rpc: Some(rpc_url),
        ..InvocationOverrides::default()
    })
    .await;

    let err = app
        .active_chain_client()
        .await
        .expect_err("reject mismatched cli rpc override");
    server.abort();

    assert!(matches!(
        err,
        Error::RpcChainIdMismatch {
            actual: 1,
            chain,
            expected: 8453,
        } if chain == "base"
    ));
}

#[tokio::test]
async fn active_chain_client_rejects_mismatched_persisted_chain_rpc() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(1).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_default_chain(&app, "base").await;
    set_rpc_config(
        &app,
        "base",
        ChainRpcConfig {
            default_rpc: rpc_url,
            rpc_urls: Vec::new(),
        },
    )
    .await;

    let err = app
        .active_chain_client()
        .await
        .expect_err("reject mismatched persisted rpc");
    server.abort();

    assert!(matches!(
        err,
        Error::RpcChainIdMismatch {
            actual: 1,
            chain,
            expected: 8453,
        } if chain == "base"
    ));
}

#[tokio::test]
async fn repl_rpc_override_rejects_mismatched_chain_id() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(1).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let mut overrides = InvocationOverrides {
        chain: Some("base".to_string()),
        ..InvocationOverrides::default()
    };

    let err = set_repl_rpc_override(&app, &mut overrides, Some(&rpc_url))
        .await
        .expect_err("reject mismatched repl rpc override");
    server.abort();

    assert!(matches!(
        err,
        Error::RpcChainIdMismatch {
            actual: 1,
            chain,
            expected: 8453,
        } if chain == "base"
    ));
    assert_eq!(overrides.rpc, None);
}

#[tokio::test]
async fn active_address_rejects_from_ens_selector_with_mismatched_ethereum_rpc() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(8453).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some("alice.eth".to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    set_ethereum_rpc(&app, &rpc_url).await;

    let err = app
        .active_address()
        .await
        .expect_err("reject mismatched ethereum rpc for ens sender");
    server.abort();

    assert_ethereum_chain_id_mismatch(err);
}

#[tokio::test]
async fn resolve_wallet_or_address_rejects_ens_selector_with_mismatched_ethereum_rpc() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(8453).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;

    let err = app
        .resolve_wallet_or_address("alice.eth")
        .await
        .expect_err("reject mismatched ethereum rpc for ens recipient");
    server.abort();

    assert_ethereum_chain_id_mismatch(err);
}

#[tokio::test]
async fn rename_wallet_rejects_ens_validation_with_mismatched_ethereum_rpc() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(8453).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;
    seed_wallets(&app, &[("alice", ALICE_ADDRESS)]).await;

    let err = rename_wallet(&app, "alice", "alice.eth")
        .await
        .expect_err("reject mismatched ethereum rpc for ens wallet-name validation");
    server.abort();

    assert_ethereum_chain_id_mismatch(err);
}
