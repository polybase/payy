// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use contracts::{Address, Client};
use serde_json::json;
use web3::ethabi::StateMutability;

use super::token_report::{load_token_balance_report, render_token_balance_report};
use crate::{
    abi::parse_function,
    cli::{TokenAction, TokenAddArgs},
    config::BeamConfig,
    error::{Error, Result},
    evm::{call_function, erc20_decimals, validate_unit_decimals},
    human_output::sanitize_control_chars,
    known_tokens::{KnownToken, token_label_key},
    output::{CommandOutput, with_loading},
    prompts::prompt_required,
    runtime::{BeamApp, ResolvedToken, parse_address},
    util::bytes::parse_bytes32_string,
};

pub async fn run(app: &BeamApp, action: Option<TokenAction>) -> Result<()> {
    match action {
        None | Some(TokenAction::List) => list_tokens(app).await,
        Some(TokenAction::Add(args)) => add_token(app, args).await,
        Some(TokenAction::Remove { token }) => remove_token(app, &token).await,
    }
}

pub(crate) async fn list_tokens(app: &BeamApp) -> Result<()> {
    render_token_balance_report(&load_token_balance_report(app).await?).print(app.output_mode)
}

pub(crate) async fn resolve_erc20_metadata(
    client: &Client,
    token: &ResolvedToken,
) -> Result<(String, u8)> {
    let decimals = match token.decimals {
        Some(decimals) => decimals,
        None => erc20_decimals(client, token.address).await?,
    };
    let address = format!("{:#x}", token.address);
    let label = if token.label.eq_ignore_ascii_case(&address) {
        lookup_token_label(client, token.address)
            .await
            .unwrap_or(address)
    } else {
        token.label.clone()
    };

    Ok((label, decimals))
}

pub(crate) fn is_native_selector(selector: &str, native_symbol: &str) -> bool {
    selector.eq_ignore_ascii_case("native") || selector.eq_ignore_ascii_case(native_symbol)
}

async fn add_token(app: &BeamApp, args: TokenAddArgs) -> Result<()> {
    let chain = app.active_chain().await?;
    let chain_key = chain.entry.key.clone();
    let native_symbol = chain.entry.native_symbol.clone();
    let TokenAddArgs {
        token,
        label,
        decimals,
    } = args;
    let selector = match token {
        Some(token) => token,
        None => prompt_required("beam token address")?,
    };
    if is_native_selector(&selector, &native_symbol) {
        return Err(Error::NativeTokenAlwaysTracked { chain: chain_key });
    }

    let mut config = app.config_store.get().await;
    let (label_key, token) = match config.known_token_by_label(&chain_key, &selector) {
        Some((label_key, token)) => (label_key, token),
        None => {
            resolve_custom_token(
                app,
                &chain_key,
                &native_symbol,
                &config,
                label,
                decimals,
                &selector,
            )
            .await?
        }
    };
    let token_known_before = config
        .known_token_by_address(&chain_key, &token.address)
        .is_some();

    if !config.track_token(&chain_key, &label_key) {
        return Err(Error::TokenAlreadyTracked {
            chain: chain_key,
            token: token.label,
        });
    }
    if !token_known_before {
        config
            .known_tokens
            .entry(chain_key.clone())
            .or_default()
            .insert(label_key, token.clone());
    }

    app.config_store
        .set(config)
        .await
        .context("persist beam tracked token")?;

    let (label, token_address) = (token.label.clone(), token.address.clone());
    let display_label = sanitize_control_chars(&label);

    CommandOutput::new(
        format!("Tracking {display_label} ({token_address}) on {chain_key}"),
        json!({
            "chain": chain_key,
            "decimals": token.decimals,
            "label": label,
            "token_address": token_address,
        }),
    )
    .compact(display_label)
    .print(app.output_mode)
}

async fn remove_token(app: &BeamApp, selector: &str) -> Result<()> {
    let chain = app.active_chain().await?;
    let chain_key = chain.entry.key.clone();
    if is_native_selector(selector, &chain.entry.native_symbol) {
        return Err(Error::NativeTokenAlwaysTracked { chain: chain_key });
    }

    let mut config = app.config_store.get().await;
    let (label_key, token) =
        tracked_token_selection(&config, &chain_key, selector).ok_or_else(|| {
            Error::TokenNotTracked {
                chain: chain_key.clone(),
                token: selector.to_string(),
            }
        })?;

    if !config.untrack_token(&chain_key, &label_key) {
        return Err(Error::TokenNotTracked {
            chain: chain_key,
            token: selector.to_string(),
        });
    }

    app.config_store
        .set(config)
        .await
        .context("persist beam tracked token removal")?;

    let (label, token_address) = (token.label.clone(), token.address.clone());
    let display_label = sanitize_control_chars(&label);

    CommandOutput::new(
        format!("Stopped tracking {display_label} ({token_address}) on {chain_key}"),
        json!({
            "chain": chain_key,
            "decimals": token.decimals,
            "label": label,
            "token_address": token_address,
        }),
    )
    .compact(display_label)
    .print(app.output_mode)
}

async fn resolve_custom_token(
    app: &BeamApp,
    chain_key: &str,
    native_symbol: &str,
    config: &BeamConfig,
    label_override: Option<String>,
    decimals_override: Option<u8>,
    selector: &str,
) -> Result<(String, KnownToken)> {
    let address = parse_address(selector)?;
    let address_value = format!("{address:#x}");
    if let Some((label_key, token)) = config.known_token_by_address(chain_key, &address_value) {
        return Ok((label_key, token));
    }

    let (_, client) = app.active_chain_client().await?;
    let (suggested_label, decimals) = with_loading(
        app.output_mode,
        format!("Fetching token metadata for {address:#x}..."),
        async {
            let decimals = match decimals_override {
                Some(decimals) => decimals,
                None => erc20_decimals(&client, address).await?,
            };
            validate_unit_decimals(usize::from(decimals))?;

            Ok::<_, Error>((lookup_token_label(&client, address).await.ok(), decimals))
        },
    )
    .await?;
    let label = match label_override.or(suggested_label) {
        Some(label) => normalize_token_label(&label)?,
        None => normalize_token_label(&prompt_required("beam token label")?)?,
    };
    let label_key = token_label_key(&label);
    if label_key == token_label_key(native_symbol) || label_key == token_label_key("native") {
        return Err(Error::ReservedTokenLabel {
            chain: chain_key.to_string(),
            label,
        });
    }
    if config.known_token_by_label(chain_key, &label).is_some() {
        return Err(Error::TokenLabelAlreadyExists {
            chain: chain_key.to_string(),
            label,
        });
    }

    Ok((
        label_key,
        KnownToken {
            address: address_value,
            decimals,
            label,
        },
    ))
}

fn tracked_token_selection(
    config: &BeamConfig,
    chain_key: &str,
    selector: &str,
) -> Option<(String, KnownToken)> {
    let tracked = config.tracked_token_keys_for_chain(chain_key);
    let selected = if let Ok(address) = parse_address(selector) {
        config.known_token_by_address(chain_key, &format!("{address:#x}"))
    } else {
        config.known_token_by_label(chain_key, selector)
    }?;

    tracked
        .into_iter()
        .any(|label| label == selected.0)
        .then_some(selected)
}

pub(crate) async fn lookup_token_label(client: &Client, token: Address) -> Result<String> {
    match lookup_token_text(client, token, "symbol").await {
        Ok(label) => Ok(label),
        Err(_) => lookup_token_text(client, token, "name").await,
    }
}

fn normalize_token_label(label: &str) -> Result<String> {
    let label = sanitize_control_chars(label);
    let label = label.trim();
    (!label.is_empty())
        .then(|| label.to_string())
        .ok_or(Error::TokenLabelBlank)
}

async fn lookup_token_text(client: &Client, token: Address, method: &str) -> Result<String> {
    if let Ok(value) = lookup_token_value(client, token, method, "string").await
        && let Ok(value) = normalize_token_label(&value)
    {
        return Ok(value);
    }

    let value = lookup_token_value(client, token, method, "bytes32").await?;
    normalize_token_label(&parse_bytes32_string(&value)?)
}

async fn lookup_token_value(
    client: &Client,
    token: Address,
    method: &str,
    output: &str,
) -> Result<String> {
    let signature = format!("{method}():({output})");
    let function = parse_function(&signature, StateMutability::View)?;
    let outcome = call_function(client, None, token, &function, &[]).await?;
    let decoded = outcome
        .decoded
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: signature.clone(),
        })?;

    decoded[0]
        .as_str()
        .map(ToString::to_string)
        .ok_or(Error::InvalidFunctionSignature { signature })
}
