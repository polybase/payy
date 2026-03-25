use super::fixtures::test_app_with_output;
use crate::{
    chains::{BeamChains, ConfiguredChain},
    commands::interactive::{handle_parsed_line, parse_line, prompt},
    config::ChainRpcConfig,
    keystore::{KeyStore, StoredKdf, StoredWallet},
    output::OutputMode,
    runtime::{BeamApp, InvocationOverrides},
};

const ALICE_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
const BEAM_DEV_RPC: &str = "https://beam.example/beam-dev";

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

fn session_app(app: &BeamApp, overrides: &InvocationOverrides) -> BeamApp {
    BeamApp {
        overrides: overrides.clone(),
        ..app.clone()
    }
}

#[tokio::test]
async fn interactive_wallet_rename_repairs_selected_wallet_override() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let mut overrides = InvocationOverrides {
        from: Some("alice".to_string()),
        ..InvocationOverrides::default()
    };
    seed_wallets(&app, &[("alice", ALICE_ADDRESS)]).await;

    handle_parsed_line(
        &app,
        &mut overrides,
        parse_line("wallets rename alice primary").expect("parse wallet rename"),
    )
    .await
    .expect("rename wallet through repl");

    assert_eq!(overrides.from.as_deref(), Some("primary"));
    prompt(&session_app(&app, &overrides))
        .await
        .expect("render prompt after wallet rename");
}

#[tokio::test]
async fn interactive_chain_remove_clears_invalid_chain_and_rpc_overrides() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let mut overrides = InvocationOverrides {
        chain: Some("beam-dev".to_string()),
        rpc: Some(BEAM_DEV_RPC.to_string()),
        ..InvocationOverrides::default()
    };
    let mut config = app.config_store.get().await;
    config.rpc_configs.insert(
        "beam-dev".to_string(),
        ChainRpcConfig::new(BEAM_DEV_RPC.to_string()),
    );
    app.config_store.set(config).await.expect("persist config");
    app.chain_store
        .set(BeamChains {
            chains: vec![ConfiguredChain {
                aliases: Vec::new(),
                chain_id: 31337,
                name: "Beam Dev".to_string(),
                native_symbol: "BEAM".to_string(),
            }],
        })
        .await
        .expect("persist chains");

    handle_parsed_line(
        &app,
        &mut overrides,
        parse_line("chains remove beam-dev").expect("parse chain removal"),
    )
    .await
    .expect("remove active chain through repl");

    assert_eq!(overrides.chain, None);
    assert_eq!(overrides.rpc, None);
    prompt(&session_app(&app, &overrides))
        .await
        .expect("render prompt after chain removal");
}

#[tokio::test]
async fn interactive_rpc_remove_clears_removed_session_rpc_override() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let mut overrides = InvocationOverrides {
        chain: Some("base".to_string()),
        rpc: Some("https://beam.example/base-1".to_string()),
        ..InvocationOverrides::default()
    };
    let mut config = app.config_store.get().await;
    config.rpc_configs.insert(
        "base".to_string(),
        ChainRpcConfig {
            default_rpc: "https://beam.example/base-1".to_string(),
            rpc_urls: vec![
                "https://beam.example/base-1".to_string(),
                "https://beam.example/base-2".to_string(),
            ],
        },
    );
    app.config_store.set(config).await.expect("persist config");

    handle_parsed_line(
        &app,
        &mut overrides,
        parse_line("rpc remove https://beam.example/base-1").expect("parse rpc removal"),
    )
    .await
    .expect("remove active rpc through repl");

    assert_eq!(overrides.chain.as_deref(), Some("base"));
    assert_eq!(overrides.rpc, None);
    prompt(&session_app(&app, &overrides))
        .await
        .expect("render prompt after rpc removal");
}

#[tokio::test]
async fn interactive_prompt_renders_with_non_ascii_rpc_url() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let overrides = InvocationOverrides {
        chain: Some("base".to_string()),
        ..InvocationOverrides::default()
    };
    let mut config = app.config_store.get().await;
    config.rpc_configs.insert(
        "base".to_string(),
        ChainRpcConfig::new("https://例え.example/路径/交易/éééééééé".to_string()),
    );
    app.config_store.set(config).await.expect("persist config");

    prompt(&session_app(&app, &overrides))
        .await
        .expect("render prompt with non-ascii rpc url");
}
