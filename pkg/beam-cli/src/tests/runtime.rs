// lint-long-file-override allow-max-lines=300
use super::{
    ens::{set_ethereum_rpc, spawn_ens_rpc_server},
    fixtures::test_app,
};
use crate::{
    chains::{BeamChains, ConfiguredChain},
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

async fn set_custom_chains(app: &BeamApp, chains: BeamChains) {
    app.chain_store
        .set(chains)
        .await
        .expect("persist custom chains");
}

async fn seed_wallets(app: &BeamApp, default_wallet: Option<&str>, wallets: &[(&str, &str)]) {
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

    let default_wallet = default_wallet.map(ToString::to_string);
    app.config_store
        .update(move |config| config.default_wallet = default_wallet.clone())
        .await
        .expect("persist default wallet");
}

#[tokio::test]
async fn active_chain_uses_selected_chain_default_rpc() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_default_chain(&app, "base").await;
    set_rpc_config(
        &app,
        "base",
        ChainRpcConfig {
            default_rpc: "https://beam.example/base-default".to_string(),
            rpc_urls: vec!["https://beam.example/base-default".to_string()],
        },
    )
    .await;

    let chain = app.active_chain().await.expect("resolve active chain");

    assert_eq!(chain.entry.key, "base");
    assert_eq!(chain.rpc_url, "https://beam.example/base-default");
}

#[tokio::test]
async fn active_chain_uses_selected_override_chain_rpc() {
    let (_temp_dir, app) = test_app(InvocationOverrides {
        chain: Some("ethereum".to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    set_default_chain(&app, "base").await;
    set_rpc_config(
        &app,
        "ethereum",
        ChainRpcConfig {
            default_rpc: "https://beam.example/ethereum-default".to_string(),
            rpc_urls: vec!["https://beam.example/ethereum-default".to_string()],
        },
    )
    .await;

    let chain = app.active_chain().await.expect("resolve active chain");

    assert_eq!(chain.entry.key, "ethereum");
    assert_eq!(chain.rpc_url, "https://beam.example/ethereum-default");
}

#[tokio::test]
async fn active_chain_client_rejects_invalid_cli_rpc_override() {
    let (_temp_dir, app) = test_app(InvocationOverrides {
        rpc: Some("foo".to_string()),
        ..InvocationOverrides::default()
    })
    .await;

    let err = app
        .active_chain_client()
        .await
        .expect_err("reject invalid cli rpc");

    assert!(matches!(err, Error::InvalidRpcUrl { value } if value == "foo"));
}

#[tokio::test]
async fn active_chain_client_rejects_invalid_persisted_chain_rpc() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_default_chain(&app, "base").await;
    set_rpc_config(
        &app,
        "base",
        ChainRpcConfig {
            default_rpc: "foo".to_string(),
            rpc_urls: vec!["foo".to_string()],
        },
    )
    .await;

    let err = app
        .active_chain_client()
        .await
        .expect_err("reject invalid persisted rpc");

    assert!(matches!(err, Error::InvalidRpcUrl { value } if value == "foo"));
}

#[tokio::test]
async fn active_chain_rejects_custom_chain_without_rpc_config() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_custom_chains(
        &app,
        BeamChains {
            chains: vec![ConfiguredChain {
                aliases: Vec::new(),
                chain_id: 31337,
                name: "Beam Dev".to_string(),
                native_symbol: "BEAM".to_string(),
            }],
        },
    )
    .await;

    let mut config = app.config_store.get().await;
    config.default_chain = "beam-dev".to_string();
    config.rpc_configs.remove("beam-dev");
    app.config_store.set(config).await.expect("persist config");

    let err = app
        .active_chain()
        .await
        .expect_err("missing custom chain rpc config");

    assert!(matches!(err, Error::NoRpcConfigured { chain } if chain == "beam-dev"));
}

#[tokio::test]
async fn active_wallet_accepts_from_address_selector() {
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some(ALICE_ADDRESS.to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    seed_wallets(&app, None, &[("alice", ALICE_ADDRESS)]).await;

    let wallet = app
        .active_wallet()
        .await
        .expect("resolve wallet by address");

    assert_eq!(wallet.name, "alice");
    assert_eq!(wallet.address, ALICE_ADDRESS);
}

#[tokio::test]
async fn active_address_accepts_raw_from_address_without_keystore_wallet() {
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some(ALICE_ADDRESS.to_string()),
        ..InvocationOverrides::default()
    })
    .await;

    let address = app.active_address().await.expect("resolve raw address");

    assert_eq!(format!("{address:#x}"), ALICE_ADDRESS);
}

#[tokio::test]
async fn active_address_accepts_from_ens_selector() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some("alice.eth".to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    set_ethereum_rpc(&app, &rpc_url).await;

    let address = app.active_address().await.expect("resolve ens name");
    server.abort();

    assert_eq!(format!("{address:#x}"), ALICE_ADDRESS);
}

#[tokio::test]
async fn active_wallet_accepts_from_ens_selector() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some("alice.eth".to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    set_ethereum_rpc(&app, &rpc_url).await;
    seed_wallets(&app, None, &[("primary", ALICE_ADDRESS)]).await;

    let wallet = app
        .active_wallet()
        .await
        .expect("resolve ens-backed wallet");
    server.abort();

    assert_eq!(wallet.name, "primary");
    assert_eq!(wallet.address, ALICE_ADDRESS);
}

#[tokio::test]
async fn resolve_wallet_or_address_uses_wallet_name() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_wallets(&app, None, &[("alice", ALICE_ADDRESS)]).await;

    let address = app
        .resolve_wallet_or_address("alice")
        .await
        .expect("resolve wallet name");

    assert_eq!(format!("{address:#x}"), ALICE_ADDRESS);
}

#[tokio::test]
async fn active_chain_uses_builtin_default_rpc_config_by_default() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let config = app.config_store.get().await;

    let chain = app.active_chain().await.expect("resolve active chain");

    assert_eq!(chain.entry.key, "ethereum");
    assert_eq!(chain.rpc_url, config.rpc_configs["ethereum"].default_rpc);
}

#[tokio::test]
async fn token_for_chain_uses_bnb_known_token_decimals_from_default_config() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;

    let usdc = app
        .token_for_chain("USDC", "bnb")
        .await
        .expect("resolve bnb usdc");
    let usdt = app
        .token_for_chain("USDT", "bnb")
        .await
        .expect("resolve bnb usdt");

    assert_eq!(usdc.decimals, Some(18));
    assert_eq!(usdt.decimals, Some(18));
}
