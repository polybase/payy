// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use serde_json::json;

use crate::{
    chains::{BeamChains, ChainEntry, all_chains, ensure_rpc_matches_chain_id, find_chain},
    cli::{RpcAction, RpcAddArgs},
    error::{Error, Result},
    human_output::sanitize_control_chars,
    output::{CommandOutput, with_loading},
    prompts::prompt_required,
    runtime::BeamApp,
    table::{render_markdown_table, render_table},
};

pub async fn run(app: &BeamApp, action: RpcAction) -> Result<()> {
    match action {
        RpcAction::List => list_rpcs(app).await,
        RpcAction::Add(args) => add_rpc(app, args).await,
        RpcAction::Remove { rpc } => remove_rpc(app, &rpc).await,
        RpcAction::Use { rpc } => use_rpc(app, &rpc).await,
    }
}

pub(crate) async fn add_rpc(app: &BeamApp, args: RpcAddArgs) -> Result<()> {
    let chain = selected_chain(app).await?;
    let rpc_url = match args.rpc {
        Some(rpc_url) => rpc_url,
        None => prompt_required("beam rpc url")?,
    };
    with_loading(
        app.output_mode,
        format!("Validating RPC {rpc_url}..."),
        async { ensure_rpc_matches_chain(&chain, &rpc_url).await },
    )
    .await?;

    let mut config = app.config_store.get().await;
    let mut rpc_config =
        config
            .rpc_config_for_chain(&chain)
            .ok_or_else(|| Error::NoRpcConfigured {
                chain: chain.key.clone(),
            })?;
    if !rpc_config.add_rpc(&rpc_url) {
        return Err(Error::RpcAlreadyExists {
            chain: chain.key.clone(),
            rpc: rpc_url,
        });
    }

    config.rpc_configs.insert(chain.key.clone(), rpc_config);
    app.config_store
        .set(config)
        .await
        .context("persist beam rpc config")?;

    CommandOutput::new(
        format!(
            "Added RPC for {}: {rpc_url}",
            sanitize_control_chars(&chain.display_name)
        ),
        json!({
            "chain": chain.key,
            "rpc_url": rpc_url,
        }),
    )
    .compact(rpc_url)
    .print(app.output_mode)
}

pub(crate) async fn remove_rpc(app: &BeamApp, rpc_url: &str) -> Result<()> {
    let chain = selected_chain(app).await?;
    let mut config = app.config_store.get().await;
    let mut rpc_config =
        config
            .rpc_config_for_chain(&chain)
            .ok_or_else(|| Error::NoRpcConfigured {
                chain: chain.key.clone(),
            })?;
    let rpc_urls = rpc_config.rpc_urls();
    if rpc_urls.iter().all(|configured| configured != rpc_url) {
        return Err(Error::RpcNotConfigured {
            chain: chain.key.clone(),
            rpc: rpc_url.to_string(),
        });
    }
    if rpc_urls.len() == 1 {
        return Err(Error::ChainRequiresRpc {
            chain: chain.key.clone(),
        });
    }

    rpc_config.remove_rpc(rpc_url);
    let default_rpc = rpc_config.default_rpc.clone();
    config.rpc_configs.insert(chain.key.clone(), rpc_config);
    app.config_store
        .set(config)
        .await
        .context("persist beam rpc removal")?;

    CommandOutput::new(
        format!(
            "Removed RPC for {}: {rpc_url}\nDefault RPC: {default_rpc}",
            sanitize_control_chars(&chain.display_name)
        ),
        json!({
            "chain": chain.key,
            "default_rpc": default_rpc,
            "removed_rpc": rpc_url,
        }),
    )
    .compact(default_rpc)
    .print(app.output_mode)
}

pub(crate) async fn use_rpc(app: &BeamApp, rpc_url: &str) -> Result<()> {
    let chain = selected_chain(app).await?;
    let mut config = app.config_store.get().await;
    let mut rpc_config =
        config
            .rpc_config_for_chain(&chain)
            .ok_or_else(|| Error::NoRpcConfigured {
                chain: chain.key.clone(),
            })?;
    if rpc_config
        .rpc_urls()
        .iter()
        .all(|configured| configured != rpc_url)
    {
        return Err(Error::RpcNotConfigured {
            chain: chain.key.clone(),
            rpc: rpc_url.to_string(),
        });
    }
    with_loading(
        app.output_mode,
        format!("Validating RPC {rpc_url}..."),
        async { ensure_rpc_matches_chain(&chain, rpc_url).await },
    )
    .await?;

    rpc_config.set_default_rpc(rpc_url);
    config.rpc_configs.insert(chain.key.clone(), rpc_config);
    app.config_store
        .set(config)
        .await
        .context("persist beam default rpc")?;

    CommandOutput::new(
        format!(
            "Default RPC for {} set to {rpc_url}",
            sanitize_control_chars(&chain.display_name)
        ),
        json!({
            "chain": chain.key,
            "default_rpc": rpc_url,
        }),
    )
    .compact(rpc_url)
    .print(app.output_mode)
}

async fn list_rpcs(app: &BeamApp) -> Result<()> {
    let beam_chains = app.chain_store.get().await;
    let config = app.config_store.get().await;
    let chains = list_scope(&beam_chains, app.overrides.chain.as_deref())?;

    let mut rows = Vec::new();
    let mut values = Vec::new();

    for chain in chains {
        let rpc_config =
            config
                .rpc_config_for_chain(&chain)
                .ok_or_else(|| Error::NoRpcConfigured {
                    chain: chain.key.clone(),
                })?;

        for rpc_url in rpc_config.rpc_urls() {
            rows.push(vec![
                marker(rpc_url == rpc_config.default_rpc),
                chain.key.clone(),
                rpc_url.clone(),
            ]);
            values.push(json!({
                "chain": chain.key,
                "chain_id": chain.chain_id,
                "is_default": rpc_url == rpc_config.default_rpc,
                "rpc_url": rpc_url,
            }));
        }
    }

    if rows.is_empty() {
        return CommandOutput::message("No RPCs configured.").print(app.output_mode);
    }

    let headers = ["default", "chain", "rpc url"];
    CommandOutput::new(render_table(&headers, &rows), json!({ "rpcs": values }))
        .markdown(render_markdown_table(&headers, &rows))
        .print(app.output_mode)
}

async fn ensure_rpc_matches_chain(chain: &ChainEntry, rpc_url: &str) -> Result<()> {
    ensure_rpc_matches_chain_id(&chain.key, chain.chain_id, rpc_url).await
}

fn list_scope(beam_chains: &BeamChains, filter: Option<&str>) -> Result<Vec<ChainEntry>> {
    match filter {
        Some(selection) => Ok(vec![find_chain(selection, beam_chains)?]),
        None => Ok(all_chains(beam_chains)),
    }
}

async fn selected_chain(app: &BeamApp) -> Result<ChainEntry> {
    let config = app.config_store.get().await;
    let selection = app
        .overrides
        .chain
        .clone()
        .unwrap_or_else(|| config.default_chain.clone());
    let beam_chains = app.chain_store.get().await;

    find_chain(&selection, &beam_chains)
}

fn marker(active: bool) -> String {
    if active {
        "*".to_string()
    } else {
        String::new()
    }
}
