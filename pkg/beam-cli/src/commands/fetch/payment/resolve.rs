// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use contracts::{Address, Client, U256};
use web3::{
    ethabi::StateMutability,
    types::{Bytes, CallRequest},
};

use crate::{
    abi::{encode_input, parse_function},
    chains::{ChainEntry, ensure_client_matches_chain_id, find_chain},
    error::{Error, Result},
    evm::erc20_decimals,
    output::with_loading,
    runtime::{BeamApp, parse_address},
};

use super::{GasEstimate, PaymentAsset, PaymentAssetKind, PaymentChain};

pub(super) async fn resolve_payment_chain(
    app: &BeamApp,
    chain_hint: Option<&str>,
    chain_id_hint: Option<u64>,
) -> Result<(PaymentChain, Client)> {
    let selector = chain_id_hint
        .map(|value| value.to_string())
        .or_else(|| chain_hint.and_then(selector_from_network))
        .or_else(|| app.overrides.chain.clone());

    if selector.is_none() && app.overrides.rpc.is_none() {
        return Err(Error::FetchPaymentChainRequired);
    }

    let config = app.config_store.get().await;
    let chain_store = app.chain_store.get().await;

    if let Some(selector) = selector.clone()
        && let Ok(entry) = find_chain(&selector, &chain_store)
    {
        let rpc_url = app
            .overrides
            .rpc
            .clone()
            .or_else(|| {
                config
                    .rpc_config_for_chain(&entry)
                    .map(|rpc| rpc.default_rpc)
            })
            .ok_or_else(|| Error::NoRpcConfigured {
                chain: entry.key.clone(),
            })?;
        let client = connect_payment_client(app, &entry, &rpc_url, Some(entry.chain_id)).await?;

        return Ok((payment_chain_from_entry(&entry, entry.chain_id), client));
    }

    let rpc_url = app
        .overrides
        .rpc
        .clone()
        .ok_or_else(|| Error::UnknownChain {
            chain: selector.clone().unwrap_or_else(|| "payment".to_string()),
        })?;
    let key = selector
        .clone()
        .or_else(|| chain_hint.map(network_key))
        .unwrap_or_else(|| "payment".to_string());
    let fallback_entry = ChainEntry {
        aliases: Vec::new(),
        chain_id: chain_id_hint.unwrap_or_default(),
        display_name: key.clone(),
        is_builtin: false,
        key,
        native_symbol: "ETH".to_string(),
        privacy: None,
    };
    let client = connect_payment_client(app, &fallback_entry, &rpc_url, chain_id_hint).await?;
    let chain_id = match chain_id_hint {
        Some(chain_id) => chain_id,
        None => client
            .chain_id_contracts()
            .await
            .context("fetch beam fetch payment chain id")?
            .low_u64(),
    };
    let entry = find_chain(&chain_id.to_string(), &chain_store).unwrap_or(ChainEntry {
        chain_id,
        ..fallback_entry
    });

    Ok((payment_chain_from_entry(&entry, chain_id), client))
}

pub(super) async fn resolve_payment_asset(
    app: &BeamApp,
    client: &Client,
    chain: &PaymentChain,
    asset_id: &str,
) -> Result<PaymentAsset> {
    if is_native_asset(asset_id, &chain.native_symbol) {
        return Ok(PaymentAsset {
            decimals: 18,
            kind: PaymentAssetKind::Native,
            label: chain.native_symbol.clone(),
        });
    }

    if let Ok(token) = app.token_for_chain(asset_id, &chain.key).await {
        let decimals = match token.decimals {
            Some(decimals) => decimals,
            None => erc20_decimals(client, token.address).await?,
        };
        return Ok(PaymentAsset {
            decimals,
            kind: PaymentAssetKind::Erc20(token.address),
            label: token.label,
        });
    }

    let address = parse_address(asset_id).map_err(|_| Error::FetchInvalidPaymentResponse)?;
    let decimals = erc20_decimals(client, address).await?;

    Ok(PaymentAsset {
        decimals,
        kind: PaymentAssetKind::Erc20(address),
        label: format!("{address:#x}"),
    })
}

pub(super) async fn estimate_payment_gas(
    client: &Client,
    payer: Address,
    recipient: Address,
    amount: U256,
    asset: &PaymentAsset,
) -> Result<GasEstimate> {
    let request = match asset.kind {
        PaymentAssetKind::Native => CallRequest {
            from: Some(payer),
            to: Some(recipient),
            value: Some(amount),
            ..Default::default()
        },
        PaymentAssetKind::Erc20(token) => {
            let function =
                parse_function("transfer(address,uint256)", StateMutability::NonPayable)?;
            let args = [format!("{recipient:#x}"), amount.to_string()];
            let data = encode_input(&function, &args)?;

            CallRequest {
                data: Some(Bytes(data)),
                from: Some(payer),
                to: Some(token),
                value: Some(U256::zero()),
                ..Default::default()
            }
        }
    };
    let gas = client
        .estimate_gas(request, None)
        .await
        .context("estimate beam fetch payment gas")?;
    let gas_limit = gas + gas / 5;
    let gas_price = client
        .fast_gas_price()
        .await
        .context("fetch beam fetch payment gas price")?;

    Ok(GasEstimate {
        fee: gas_limit * gas_price,
        gas_limit,
        gas_price,
    })
}

pub(super) fn chain_id_from_network(network: &str) -> Option<u64> {
    network
        .trim()
        .strip_prefix("eip155:")
        .and_then(|value| value.parse::<u64>().ok())
}

async fn connect_payment_client(
    app: &BeamApp,
    entry: &ChainEntry,
    rpc_url: &str,
    expected_chain_id: Option<u64>,
) -> Result<Client> {
    let rpc_url = rpc_url.to_string();
    let key = entry.key.clone();
    let client = with_loading(
        app.output_mode,
        format!("Connecting to {} RPC...", key),
        async move {
            Client::try_new(&rpc_url, None).map_err(|_| Error::InvalidRpcUrl {
                value: rpc_url.clone(),
            })
        },
    )
    .await?;

    if let Some(expected_chain_id) = expected_chain_id {
        ensure_client_matches_chain_id(&entry.key, expected_chain_id, &client).await?;
    }

    Ok(client)
}

fn selector_from_network(network: &str) -> Option<String> {
    chain_id_from_network(network)
        .map(|value| value.to_string())
        .or_else(|| (!network.trim().is_empty()).then(|| network.trim().to_string()))
}

fn network_key(network: &str) -> String {
    network.trim().replace([':', '_'], "-").to_ascii_lowercase()
}

fn is_native_asset(asset_id: &str, native_symbol: &str) -> bool {
    let normalized = asset_id.trim().to_ascii_lowercase();
    normalized == "native"
        || normalized == native_symbol.to_ascii_lowercase()
        || normalized == "0x0000000000000000000000000000000000000000"
        || normalized == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
}

pub(super) fn payment_chain_from_entry(entry: &ChainEntry, chain_id: u64) -> PaymentChain {
    PaymentChain {
        aliases: entry.aliases.clone(),
        chain_id,
        display_name: entry.display_name.clone(),
        key: entry.key.clone(),
        native_symbol: entry.native_symbol.clone(),
        privacy: entry.privacy.clone(),
    }
}
