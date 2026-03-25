use std::time::Duration;

use contextful::ResultContextExt;
use contracts::{Address, Client};
use web3::{api::Namespace, contract::ens::Ens};

use crate::{
    chains::{ensure_client_matches_chain_id, find_chain},
    error::{Error, Result},
    keystore::{KeyStore, StoredWallet, next_wallet_name, validate_wallet_name},
    output::with_loading,
    runtime::BeamApp,
};

const ETHEREUM_CHAIN_ID: u64 = 1;
const ETHEREUM_CHAIN_KEY: &str = "ethereum";
const ENS_LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn import_wallet_name(
    app: &BeamApp,
    keystore: &KeyStore,
    requested_name: Option<String>,
    address: Address,
) -> Result<String> {
    if let Some(name) = requested_name {
        return Ok(name);
    }

    if let Some(name) = best_effort_verified_ens_name(app, address).await?
        && validate_wallet_name(&keystore.wallets, &name, None).is_ok()
    {
        return Ok(name);
    }

    Ok(next_wallet_name(keystore))
}

pub fn ens_name(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase();
    (normalized.len() > ".eth".len() && normalized.ends_with(".eth")).then_some(normalized)
}

pub async fn lookup_ens_address(client: &Client, name: &str) -> Result<Option<Address>> {
    ensure_ethereum_mainnet_ens_client(client).await?;
    lookup_ens_address_unchecked(client, name).await
}

pub async fn lookup_verified_ens_name(client: &Client, address: Address) -> Result<Option<String>> {
    ensure_ethereum_mainnet_ens_client(client).await?;

    let ens = Ens::new(client.client().transport().clone());
    let name = ens
        .canonical_name(address)
        .await
        .context("resolve beam wallet ens reverse record")?;
    let name = name.trim();
    if name.is_empty() {
        return Ok(None);
    }

    // Only trust reverse records that resolve back to the original address.
    let resolved = lookup_ens_address_unchecked(client, name).await?;
    if resolved != Some(address) {
        return Ok(None);
    }

    Ok(Some(name.to_string()))
}

async fn lookup_ens_address_unchecked(client: &Client, name: &str) -> Result<Option<Address>> {
    let ens = Ens::new(client.client().transport().clone());
    let resolver = ens
        .resolver(name)
        .await
        .context("resolve beam ens resolver")?;
    if resolver == Address::zero() {
        return Ok(None);
    }
    let address = match ens.eth_address(name).await {
        Ok(address) => address,
        Err(web3::contract::Error::InterfaceUnsupported) => return Ok(None),
        Err(error) => Err(error).context("resolve beam ens forward record")?,
    };

    if address == Address::zero() {
        return Ok(None);
    }

    Ok(Some(address))
}

pub async fn resolve_ens_address(app: &BeamApp, name: &str) -> Result<Option<Address>> {
    with_loading(
        app.output_mode,
        format!("Resolving ENS name {name}..."),
        async {
            let client = ethereum_client(app).await?;
            lookup_ens_address(&client, name).await
        },
    )
    .await
}

pub async fn validate_wallet_name_for_address(
    app: &BeamApp,
    wallets: &[StoredWallet],
    name: &str,
    current_address: Option<&str>,
    address: &str,
) -> Result<()> {
    validate_wallet_name(wallets, name, current_address)?;

    let Some(name) = ens_name(name) else {
        return Ok(());
    };
    let expected = address.parse().map_err(|_| Error::InvalidAddress {
        value: address.to_string(),
    })?;
    let resolved = resolve_ens_address(app, &name)
        .await?
        .ok_or_else(|| Error::EnsNameNotFound { name: name.clone() })?;
    if resolved != expected {
        return Err(Error::WalletNameEnsAddressMismatch {
            address: address.to_string(),
            name,
        });
    }

    Ok(())
}

async fn best_effort_verified_ens_name(app: &BeamApp, address: Address) -> Result<Option<String>> {
    let client = match ethereum_client(app).await {
        Ok(client) => client,
        Err(_) => return Ok(None),
    };

    with_loading(
        app.output_mode,
        format!("Looking up ENS name for {address:#x}..."),
        async {
            match tokio::time::timeout(
                ENS_LOOKUP_TIMEOUT,
                lookup_verified_ens_name(&client, address),
            )
            .await
            {
                Ok(Ok(name)) => Ok(name),
                Ok(Err(_)) | Err(_) => Ok(None),
            }
        },
    )
    .await
}

async fn ethereum_client(app: &BeamApp) -> Result<Client> {
    let chains = app.chain_store.get().await;
    let config = app.config_store.get().await;
    let ethereum = find_chain(ETHEREUM_CHAIN_KEY, &chains)?;
    let rpc_url = config
        .rpc_config_for_chain(&ethereum)
        .map(|rpc_config| rpc_config.default_rpc)
        .ok_or_else(|| Error::NoRpcConfigured {
            chain: ethereum.key.clone(),
        })?;

    Client::try_new(&rpc_url, None).map_err(|_| Error::InvalidRpcUrl { value: rpc_url })
}

async fn ensure_ethereum_mainnet_ens_client(client: &Client) -> Result<()> {
    ensure_client_matches_chain_id(ETHEREUM_CHAIN_KEY, ETHEREUM_CHAIN_ID, client).await
}
