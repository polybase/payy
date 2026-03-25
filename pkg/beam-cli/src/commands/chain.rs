// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use serde_json::json;

use crate::{
    chains::{
        BeamChains, ConfiguredChain, all_chains, chain_key, ensure_rpc_matches_chain_id,
        find_chain, resolve_rpc_chain_id,
    },
    cli::{ChainAction, ChainAddArgs},
    config::ChainRpcConfig,
    error::{Error, Result},
    human_output::{sanitize_control_chars, sanitize_control_chars_trimmed},
    output::{CommandOutput, with_loading},
    prompts::{prompt_required, prompt_with_default},
    runtime::BeamApp,
    table::{render_markdown_table, render_table},
};

const DEFAULT_CHAIN_KEY: &str = "ethereum";
const DEFAULT_NATIVE_SYMBOL: &str = "ETH";

pub async fn run(app: &BeamApp, action: ChainAction) -> Result<()> {
    match action {
        ChainAction::List => list_chains(app).await,
        ChainAction::Add(args) => add_chain(app, args).await,
        ChainAction::Remove { chain } => remove_chain(app, &chain).await,
        ChainAction::Use { chain } => use_chain(app, &chain).await,
    }
}

pub(crate) async fn add_chain(app: &BeamApp, args: ChainAddArgs) -> Result<()> {
    let ChainAddArgs {
        name,
        rpc,
        chain_id,
        native_symbol,
    } = args;
    let interactive_native_symbol = native_symbol.is_none() && (name.is_none() || rpc.is_none());
    let name = normalize_chain_name(match name {
        Some(name) => name,
        None => prompt_required("beam chain name")?,
    })?;
    let rpc_url = match rpc {
        Some(rpc_url) => rpc_url,
        None => prompt_required("beam chain rpc")?,
    };
    let mut beam_chains = app.chain_store.get().await;
    validate_new_chain_name(&name, &beam_chains)?;
    let key = chain_key(&name);
    let chain_id = with_loading(
        app.output_mode,
        format!("Validating RPC {rpc_url}..."),
        async {
            match chain_id {
                Some(chain_id) => {
                    ensure_rpc_matches_chain_id(&key, chain_id, &rpc_url).await?;
                    Ok(chain_id)
                }
                None => resolve_rpc_chain_id(&rpc_url).await,
            }
        },
    )
    .await?;

    let native_symbol = normalize_native_symbol(match native_symbol {
        Some(native_symbol) => Some(native_symbol),
        None if interactive_native_symbol => Some(prompt_with_default(
            "beam chain native symbol",
            DEFAULT_NATIVE_SYMBOL,
        )?),
        None => None,
    });
    let configured_chain = ConfiguredChain {
        aliases: Vec::new(),
        chain_id,
        name: name.clone(),
        native_symbol: native_symbol.clone(),
    };

    let existing = all_chains(&beam_chains);
    if existing.iter().any(|chain| chain.chain_id == chain_id) {
        return Err(Error::ChainIdAlreadyExists { chain_id });
    }

    let mut config = app.config_store.get().await;
    config
        .rpc_configs
        .insert(key.clone(), ChainRpcConfig::new(rpc_url.clone()));
    beam_chains.chains.push(configured_chain);

    app.config_store
        .set(config)
        .await
        .context("persist beam chain rpc config")?;
    app.chain_store
        .set(beam_chains)
        .await
        .context("persist beam chains")?;

    CommandOutput::new(
        format!(
            "Added chain {} ({}, id {chain_id}) with default RPC {rpc_url}",
            sanitize_control_chars(&name),
            sanitize_control_chars(&key)
        ),
        json!({
            "chain": key,
            "chain_id": chain_id,
            "default_rpc": rpc_url,
            "name": name,
            "native_symbol": native_symbol,
        }),
    )
    .compact(format!("{} {chain_id}", sanitize_control_chars(&key)))
    .print(app.output_mode)
}

pub(crate) async fn remove_chain(app: &BeamApp, selection: &str) -> Result<()> {
    let mut beam_chains = app.chain_store.get().await;
    let chain = find_chain(selection, &beam_chains)?;
    if chain.is_builtin {
        return Err(Error::BuiltinChainRemovalNotAllowed {
            chain: chain.key.clone(),
        });
    }

    beam_chains
        .chains
        .retain(|configured| chain_key(&configured.name) != chain.key);

    let mut config = app.config_store.get().await;
    config.rpc_configs.remove(&chain.key);
    config.known_tokens.remove(&chain.key);
    config.tracked_tokens.remove(&chain.key);
    if config.default_chain == chain.key {
        config.default_chain = DEFAULT_CHAIN_KEY.to_string();
    }

    app.chain_store
        .set(beam_chains)
        .await
        .context("persist beam chains")?;
    app.config_store
        .set(config)
        .await
        .context("persist beam chain removal config")?;

    CommandOutput::new(
        format!(
            "Removed chain {} ({})",
            sanitize_control_chars(&chain.display_name),
            sanitize_control_chars(&chain.key)
        ),
        json!({
            "chain": chain.key,
            "chain_id": chain.chain_id,
            "name": chain.display_name,
        }),
    )
    .compact(sanitize_control_chars(&chain.key))
    .print(app.output_mode)
}

pub(crate) async fn use_chain(app: &BeamApp, selection: &str) -> Result<()> {
    let beam_chains = app.chain_store.get().await;
    let chain = find_chain(selection, &beam_chains)?;
    let config = app.config_store.get().await;
    if config.rpc_config_for_chain(&chain).is_none() {
        return Err(Error::NoRpcConfigured {
            chain: chain.key.clone(),
        });
    }

    let mut config = config;
    config.default_chain = chain.key.clone();

    app.config_store
        .set(config)
        .await
        .context("persist beam default chain")?;

    CommandOutput::new(
        format!(
            "Default chain set to {} ({})",
            sanitize_control_chars(&chain.display_name),
            chain.chain_id
        ),
        json!({
            "chain": chain.key,
            "chain_id": chain.chain_id,
            "name": chain.display_name,
        }),
    )
    .compact(sanitize_control_chars(&chain.key))
    .print(app.output_mode)
}

async fn list_chains(app: &BeamApp) -> Result<()> {
    let beam_chains = app.chain_store.get().await;
    let chains = all_chains(&beam_chains);
    let config = app.config_store.get().await;
    let rows = chains
        .iter()
        .map(|chain| {
            vec![
                marker(config.default_chain == chain.key),
                chain.key.clone(),
                chain.display_name.clone(),
                chain.chain_id.to_string(),
                chain.native_symbol.clone(),
                config
                    .rpc_config_for_chain(chain)
                    .map(|rpc_config| rpc_config.rpc_urls().len())
                    .unwrap_or_default()
                    .to_string(),
                if chain.is_builtin {
                    "builtin".to_string()
                } else {
                    "custom".to_string()
                },
            ]
        })
        .collect::<Vec<_>>();
    let headers = ["default", "chain", "name", "id", "symbol", "rpcs", "source"];

    CommandOutput::new(
        render_table(&headers, &rows),
        json!({
            "chains": chains.iter().map(|chain| {
                json!({
                    "chain": chain.key,
                    "chain_id": chain.chain_id,
                    "is_builtin": chain.is_builtin,
                    "is_default": config.default_chain == chain.key,
                    "name": chain.display_name,
                    "native_symbol": chain.native_symbol,
                    "rpc_count": config
                        .rpc_config_for_chain(chain)
                        .map(|rpc_config| rpc_config.rpc_urls().len())
                        .unwrap_or_default(),
                })
            }).collect::<Vec<_>>()
        }),
    )
    .markdown(render_markdown_table(&headers, &rows))
    .print(app.output_mode)
}

fn marker(active: bool) -> String {
    if active {
        "*".to_string()
    } else {
        String::new()
    }
}

fn normalize_chain_name(name: String) -> Result<String> {
    let name = sanitize_control_chars_trimmed(&name);
    if name.is_empty() {
        return Err(Error::InvalidChainName { name });
    }

    Ok(name)
}

fn validate_new_chain_name(name: &str, configured: &BeamChains) -> Result<()> {
    let key = chain_key(name);
    if let Ok(existing_chain) = find_chain(&key, configured) {
        return Err(if existing_chain.key == key {
            Error::ChainNameAlreadyExists {
                name: name.to_string(),
            }
        } else {
            Error::ChainNameConflictsWithSelector {
                name: name.to_string(),
            }
        });
    }

    Ok(())
}

fn normalize_native_symbol(native_symbol: Option<String>) -> String {
    native_symbol
        .map(|value| sanitize_control_chars_trimmed(&value).to_ascii_uppercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_NATIVE_SYMBOL.to_string())
}
