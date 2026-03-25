use json_store::JsonStoreError;
use tempfile::TempDir;

use crate::{
    config::{BeamConfig, ChainRpcConfig, load_config},
    error::Error,
};

#[test]
fn defaults_payy_testnet_to_testnet_rpc() {
    let config = BeamConfig::default();

    assert_eq!(
        config.rpc_configs["payy-testnet"].default_rpc,
        "https://rpc.testnet.payy.network"
    );
}

#[test]
fn defaults_builtin_chains_to_public_rpc_urls() {
    let config = BeamConfig::default();

    let expected = [
        ("ethereum", "https://ethereum-rpc.publicnode.com"),
        ("base", "https://base-rpc.publicnode.com"),
        ("polygon", "https://polygon-bor-rpc.publicnode.com"),
        ("bnb", "https://bsc-rpc.publicnode.com"),
        ("arbitrum", "https://arbitrum-one-rpc.publicnode.com"),
        ("payy-testnet", "https://rpc.testnet.payy.network"),
        ("payy-dev", "http://127.0.0.1:8546"),
        ("sepolia", "https://ethereum-sepolia-rpc.publicnode.com"),
        ("hardhat", "http://127.0.0.1:8545"),
    ];

    for (chain_key, rpc_url) in expected {
        let rpc_config = &config.rpc_configs[chain_key];

        assert_eq!(rpc_config.default_rpc, rpc_url);
        assert_eq!(rpc_config.rpc_urls, vec![rpc_url.to_string()]);
    }
}

#[tokio::test]
async fn persists_config_updates() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let store = load_config(temp_dir.path())
        .await
        .expect("load config store");

    let config = store.get().await;
    assert_eq!(config.default_chain, "ethereum");
    assert!(config.known_tokens.contains_key("base"));
    assert_eq!(config.tracked_tokens["base"], vec!["USDC".to_string()]);
    assert!(config.rpc_configs.contains_key("base"));

    let mut rpc_configs = config.rpc_configs.clone();
    rpc_configs.insert(
        "base".to_string(),
        ChainRpcConfig {
            default_rpc: "https://beam.example/base-2".to_string(),
            rpc_urls: vec![
                "https://beam.example/base-1".to_string(),
                "https://beam.example/base-2".to_string(),
            ],
        },
    );

    store
        .set(BeamConfig {
            default_chain: "base".to_string(),
            default_wallet: Some("alice".to_string()),
            known_tokens: config.known_tokens.clone(),
            tracked_tokens: config.tracked_tokens.clone(),
            rpc_configs,
        })
        .await
        .expect("persist config");

    let reloaded = load_config(temp_dir.path())
        .await
        .expect("reload config store")
        .get()
        .await;

    assert_eq!(reloaded.default_chain, "base");
    assert_eq!(reloaded.default_wallet.as_deref(), Some("alice"));
    assert_eq!(
        reloaded.rpc_configs["base"].default_rpc,
        "https://beam.example/base-2"
    );
    assert_eq!(
        reloaded.rpc_configs["base"].rpc_urls,
        vec![
            "https://beam.example/base-1".to_string(),
            "https://beam.example/base-2".to_string(),
        ]
    );
}

#[tokio::test]
async fn rejects_invalid_persisted_config_json() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_path = temp_dir.path().join("config.json");
    std::fs::write(&file_path, "{ invalid json").expect("write invalid config");

    let err = match load_config(temp_dir.path()).await {
        Ok(_) => panic!("expected invalid config to fail"),
        Err(err) => err,
    };

    match err {
        Error::Internal(internal) => match internal.recursive_downcast_ref::<JsonStoreError>() {
            Some(JsonStoreError::Deserialization { path, .. }) => assert_eq!(path, &file_path),
            other => panic!("unexpected json store error: {other:?}"),
        },
        other => panic!("unexpected error: {other:?}"),
    }
}
