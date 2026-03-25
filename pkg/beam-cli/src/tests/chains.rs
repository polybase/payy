use super::fixtures::{spawn_chain_id_rpc_server, test_app_with_output};
use json_store::JsonStoreError;
use tempfile::TempDir;

use crate::{
    chains::{BeamChains, ConfiguredChain, find_chain, load_chains},
    cli::ChainAddArgs,
    commands::chain,
    error::Error,
    known_tokens::default_known_tokens,
    output::OutputMode,
    runtime::InvocationOverrides,
};

#[test]
fn finds_builtin_chain_by_name_and_alias() {
    let chains = BeamChains::default();

    let base = find_chain("base", &chains).expect("find base chain");
    assert_eq!(base.chain_id, 8453);

    let bnb = find_chain("bsc", &chains).expect("find bnb alias");
    assert_eq!(bnb.key, "bnb");
}

#[test]
fn finds_custom_chain_by_id() {
    let chains = BeamChains {
        chains: vec![ConfiguredChain {
            aliases: vec!["beamdev".to_string()],
            chain_id: 31337,
            name: "Beam Dev".to_string(),
            native_symbol: "BEAM".to_string(),
        }],
    };

    let chain = find_chain("31337", &chains).expect("find custom chain");
    assert_eq!(chain.display_name, "Beam Dev");
    assert_eq!(chain.key, "beam-dev");
    assert!(!chain.is_builtin);
}

#[tokio::test]
async fn chain_add_rejects_names_that_conflict_with_builtin_aliases() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    for name in ["eth", "bsc"] {
        let err = chain::add_chain(
            &app,
            ChainAddArgs {
                name: Some(name.to_string()),
                rpc: Some("https://beam.example/unused".to_string()),
                chain_id: Some(31337),
                native_symbol: Some("BEAM".to_string()),
            },
        )
        .await
        .expect_err("reject builtin alias collision");

        assert!(matches!(
            err,
            Error::ChainNameConflictsWithSelector { name: conflicted_name }
                if conflicted_name == name
        ));
    }
}

#[tokio::test]
async fn chain_add_rejects_names_that_conflict_with_numeric_selectors() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    let err = chain::add_chain(
        &app,
        ChainAddArgs {
            name: Some("1".to_string()),
            rpc: Some("https://beam.example/unused".to_string()),
            chain_id: Some(31337),
            native_symbol: Some("BEAM".to_string()),
        },
    )
    .await
    .expect_err("reject numeric selector collision");

    assert!(matches!(
        err,
        Error::ChainNameConflictsWithSelector { name } if name == "1"
    ));
}

#[tokio::test]
async fn chain_add_sanitizes_control_characters_before_persisting() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(31337).await;
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    chain::add_chain(
        &app,
        ChainAddArgs {
            name: Some("Beam\n\x1b[31m Dev".to_string()),
            rpc: Some(rpc_url),
            chain_id: Some(31337),
            native_symbol: Some("BEAM".to_string()),
        },
    )
    .await
    .expect("add chain with sanitized name");
    server.abort();

    let chains = app.chain_store.get().await;
    assert_eq!(chains.chains[0].name, "Beam ?[31m Dev");

    let chain = find_chain("beam-?[31m-dev", &chains).expect("find sanitized chain");
    assert_eq!(chain.display_name, "Beam ?[31m Dev");

    let config = app.config_store.get().await;
    assert!(config.rpc_configs.contains_key("beam-?[31m-dev"));
}

#[tokio::test]
async fn chain_add_sanitizes_control_characters_in_native_symbol_before_persisting() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(31337).await;
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    chain::add_chain(
        &app,
        ChainAddArgs {
            name: Some("Beam Dev".to_string()),
            rpc: Some(rpc_url),
            chain_id: Some(31337),
            native_symbol: Some("\x1b[31mbe\nam\t".to_string()),
        },
    )
    .await
    .expect("add chain with sanitized native symbol");
    server.abort();

    let chains = app.chain_store.get().await;
    assert_eq!(chains.chains[0].native_symbol, "?[31MBE AM");
}

#[test]
fn snapshots_default_known_tokens() {
    insta::assert_json_snapshot!(default_known_tokens());
}

#[tokio::test]
async fn rejects_invalid_persisted_chain_json() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_path = temp_dir.path().join("chains.json");
    std::fs::write(&file_path, "{ invalid json").expect("write invalid chains");

    let err = match load_chains(temp_dir.path()).await {
        Ok(_) => panic!("expected invalid chains to fail"),
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
