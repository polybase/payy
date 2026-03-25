// lint-long-file-override allow-max-lines=300
use super::fixtures::{spawn_chain_id_rpc_server, test_app_with_output};
use crate::{
    cli::{ChainAddArgs, RpcAddArgs},
    commands::{chain, rpc},
    config::ChainRpcConfig,
    error::Error,
    output::OutputMode,
    runtime::InvocationOverrides,
};

fn beam_dev_chain_args(rpc_url: String) -> ChainAddArgs {
    ChainAddArgs {
        name: Some("Beam Dev".to_string()),
        rpc: Some(rpc_url),
        chain_id: Some(31337),
        native_symbol: Some("BEAM".to_string()),
    }
}

#[tokio::test]
async fn chain_add_with_explicit_chain_id_persists_custom_chain_and_rpc() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(31337).await;
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    chain::add_chain(&app, beam_dev_chain_args(rpc_url.clone()))
        .await
        .expect("add custom chain");
    server.abort();

    let chains = app.chain_store.get().await;
    let config = app.config_store.get().await;

    assert_eq!(chains.chains.len(), 1);
    assert_eq!(chains.chains[0].name, "Beam Dev");
    assert_eq!(chains.chains[0].chain_id, 31337);
    assert_eq!(chains.chains[0].native_symbol, "BEAM");
    assert_eq!(config.rpc_configs["beam-dev"].default_rpc, rpc_url);
}

#[tokio::test]
async fn chain_add_with_explicit_chain_id_rejects_mismatched_rpc_chain_id() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(1).await;
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    let err = chain::add_chain(&app, beam_dev_chain_args(rpc_url))
        .await
        .expect_err("reject mismatched chain id");
    server.abort();

    assert!(matches!(
        err,
        Error::RpcChainIdMismatch {
            actual: 1,
            chain,
            expected: 31337,
        } if chain == "beam-dev"
    ));
    assert!(app.chain_store.get().await.chains.is_empty());
    let config = app.config_store.get().await;
    assert!(!config.rpc_configs.contains_key("beam-dev"));
}

#[tokio::test]
async fn chain_remove_drops_custom_chain_and_resets_default_chain() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(31337).await;
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    chain::add_chain(&app, beam_dev_chain_args(rpc_url))
        .await
        .expect("add custom chain");
    server.abort();
    chain::use_chain(&app, "beam-dev")
        .await
        .expect("set custom chain as default");

    chain::remove_chain(&app, "beam-dev")
        .await
        .expect("remove custom chain");

    let chains = app.chain_store.get().await;
    let config = app.config_store.get().await;

    assert!(chains.chains.is_empty());
    assert!(!config.rpc_configs.contains_key("beam-dev"));
    assert!(!config.tracked_tokens.contains_key("beam-dev"));
    assert_eq!(config.default_chain, "ethereum");
}

#[tokio::test]
async fn chain_use_updates_default_chain() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    chain::use_chain(&app, "base")
        .await
        .expect("use builtin chain");

    assert_eq!(app.config_store.get().await.default_chain, "base");
}

#[tokio::test]
async fn rpc_use_updates_default_rpc_for_selected_chain() {
    let (rpc_url_1, server_1) = spawn_chain_id_rpc_server(8453).await;
    let (rpc_url_2, server_2) = spawn_chain_id_rpc_server(8453).await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
    let mut config = app.config_store.get().await;
    config.rpc_configs.insert(
        "base".to_string(),
        ChainRpcConfig {
            default_rpc: rpc_url_1.clone(),
            rpc_urls: vec![rpc_url_1.clone(), rpc_url_2.clone()],
        },
    );
    app.config_store
        .set(config)
        .await
        .expect("persist rpc config");

    rpc::use_rpc(&app, &rpc_url_2).await.expect("use rpc");
    server_1.abort();
    server_2.abort();

    assert_eq!(
        app.config_store.get().await.rpc_configs["base"].default_rpc,
        rpc_url_2
    );
}

#[tokio::test]
async fn rpc_use_rejects_stale_configured_rpc_with_wrong_chain_id() {
    let (rpc_url, server) = spawn_chain_id_rpc_server(1).await;
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
    let mut config = app.config_store.get().await;
    config.rpc_configs.insert(
        "base".to_string(),
        ChainRpcConfig {
            default_rpc: "https://beam.example/base-1".to_string(),
            rpc_urls: vec!["https://beam.example/base-1".to_string(), rpc_url.clone()],
        },
    );
    app.config_store
        .set(config)
        .await
        .expect("persist rpc config");

    let err = rpc::use_rpc(&app, &rpc_url)
        .await
        .expect_err("reject stale mismatched rpc");
    server.abort();

    assert!(matches!(
        err,
        Error::RpcChainIdMismatch {
            actual: 1,
            chain,
            expected: 8453,
        } if chain == "base"
    ));
    assert_eq!(
        app.config_store.get().await.rpc_configs["base"].default_rpc,
        "https://beam.example/base-1"
    );
}

#[tokio::test]
async fn rpc_remove_promotes_next_default_for_selected_chain() {
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
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
    app.config_store
        .set(config)
        .await
        .expect("persist rpc config");

    rpc::remove_rpc(&app, "https://beam.example/base-1")
        .await
        .expect("remove default rpc");

    let config = app.config_store.get().await;
    assert_eq!(
        config.rpc_configs["base"].default_rpc,
        "https://beam.example/base-2"
    );
    assert_eq!(
        config.rpc_configs["base"].rpc_urls,
        vec!["https://beam.example/base-2".to_string()]
    );
}

#[test]
fn rpc_add_args_accept_missing_value_for_interactive_prompt() {
    let args = RpcAddArgs { rpc: None };
    assert!(args.rpc.is_none());
}
