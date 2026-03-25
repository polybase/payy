// lint-long-file-override allow-max-lines=300
use std::{collections::BTreeMap, path::Path};

use contextful::ResultContextExt;
use contracts::Client;
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const ETHEREUM_RPC_URL: &str = "https://ethereum-rpc.publicnode.com";
const BASE_RPC_URL: &str = "https://base-rpc.publicnode.com";
const POLYGON_RPC_URL: &str = "https://polygon-bor-rpc.publicnode.com";
const BNB_RPC_URL: &str = "https://bsc-rpc.publicnode.com";
const ARBITRUM_RPC_URL: &str = "https://arbitrum-one-rpc.publicnode.com";
const PAYY_TESTNET_RPC_URL: &str = "https://rpc.testnet.payy.network";
const PAYY_DEV_RPC_URL: &str = "http://127.0.0.1:8546";
const SEPOLIA_RPC_URL: &str = "https://ethereum-sepolia-rpc.publicnode.com";

type BuiltinChainSpec = (
    &'static str,
    &'static str,
    u64,
    &'static str,
    &'static str,
    &'static [&'static str],
);

const BUILTIN_CHAINS: [BuiltinChainSpec; 9] = [
    ("ethereum", "Ethereum", 1, "ETH", ETHEREUM_RPC_URL, &["eth"]),
    ("base", "Base", 8453, "ETH", BASE_RPC_URL, &[]),
    ("polygon", "Polygon", 137, "MATIC", POLYGON_RPC_URL, &[]),
    ("bnb", "BNB", 56, "BNB", BNB_RPC_URL, &["bsc", "binance"]),
    (
        "arbitrum",
        "Arbitrum",
        42161,
        "ETH",
        ARBITRUM_RPC_URL,
        &["arb"],
    ),
    (
        "payy-testnet",
        "Payy Testnet",
        7298,
        "PUSD",
        PAYY_TESTNET_RPC_URL,
        &["payy", "payytestnet"],
    ),
    (
        "payy-dev",
        "Payy Dev",
        7297,
        "PUSD",
        PAYY_DEV_RPC_URL,
        &["payydev"],
    ),
    ("sepolia", "Sepolia", 11155111, "ETH", SEPOLIA_RPC_URL, &[]),
    (
        "hardhat",
        "Hardhat",
        1337,
        "ETH",
        "http://127.0.0.1:8545",
        &["local"],
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainEntry {
    pub aliases: Vec<String>,
    pub chain_id: u64,
    pub display_name: String,
    pub is_builtin: bool,
    pub key: String,
    pub native_symbol: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BeamChains {
    pub chains: Vec<ConfiguredChain>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfiguredChain {
    #[serde(default)]
    pub aliases: Vec<String>,
    pub chain_id: u64,
    pub name: String,
    #[serde(default = "default_native_symbol")]
    pub native_symbol: String,
}

pub async fn load_chains(root: &Path) -> Result<JsonStore<BeamChains>> {
    let store = JsonStore::new_with_invalid_json_behavior_and_access(
        root,
        "chains.json",
        InvalidJsonBehavior::Error,
        FileAccess::OwnerOnly,
    )
    .await
    .context("load beam chains store")?;
    Ok(store)
}

pub fn all_chains(configured: &BeamChains) -> Vec<ChainEntry> {
    let mut chains = builtin_chains();
    chains.extend(configured.chains.iter().map(custom_chain_entry));
    chains
}

pub fn find_chain(selection: &str, configured: &BeamChains) -> Result<ChainEntry> {
    let chains = all_chains(configured);
    if let Ok(chain_id) = selection.parse::<u64>()
        && let Some(chain) = chains.iter().find(|entry| entry.chain_id == chain_id)
    {
        return Ok(chain.clone());
    }

    let needle = canonicalize(selection);
    let chain = chains
        .into_iter()
        .find(|entry| entry.key == needle || entry.aliases.iter().any(|alias| alias == &needle))
        .ok_or_else(|| Error::UnknownChain {
            chain: selection.to_string(),
        })?;

    Ok(chain)
}

pub fn builtin_rpc_url(chain_key: &str) -> Option<&'static str> {
    BUILTIN_CHAINS
        .iter()
        .find_map(|spec| (spec.0 == chain_key).then_some(spec.4))
}

pub fn chain_key(name: &str) -> String {
    canonicalize(name)
}

pub async fn resolve_rpc_chain_id(rpc_url: &str) -> Result<u64> {
    let client = client_for_rpc(rpc_url)?;
    resolve_client_chain_id(&client).await
}

pub async fn resolve_client_chain_id(client: &Client) -> Result<u64> {
    let chain_id = client
        .chain_id_contracts()
        .await
        .context("fetch beam chain id from rpc")?;

    Ok(chain_id.low_u64())
}

pub async fn ensure_client_matches_chain_id(
    chain_key: &str,
    expected_chain_id: u64,
    client: &Client,
) -> Result<()> {
    let actual_chain_id = resolve_client_chain_id(client).await?;
    if actual_chain_id != expected_chain_id {
        return Err(Error::RpcChainIdMismatch {
            actual: actual_chain_id,
            chain: chain_key.to_string(),
            expected: expected_chain_id,
        });
    }

    Ok(())
}

pub async fn ensure_rpc_matches_chain_id(
    chain_key: &str,
    expected_chain_id: u64,
    rpc_url: &str,
) -> Result<()> {
    let client = client_for_rpc(rpc_url)?;
    ensure_client_matches_chain_id(chain_key, expected_chain_id, &client).await
}

pub fn default_rpc_urls() -> BTreeMap<String, String> {
    BUILTIN_CHAINS
        .iter()
        .map(|spec| (spec.0.to_string(), spec.4.to_string()))
        .collect()
}

fn default_native_symbol() -> String {
    "ETH".to_string()
}

fn builtin_chains() -> Vec<ChainEntry> {
    BUILTIN_CHAINS.iter().map(builtin_entry).collect()
}

fn client_for_rpc(rpc_url: &str) -> Result<Client> {
    Client::try_new(rpc_url, None).map_err(|_| Error::InvalidRpcUrl {
        value: rpc_url.to_string(),
    })
}

fn builtin_entry(spec: &BuiltinChainSpec) -> ChainEntry {
    ChainEntry {
        aliases: spec.5.iter().map(|alias| canonicalize(alias)).collect(),
        chain_id: spec.2,
        display_name: spec.1.to_string(),
        is_builtin: true,
        key: spec.0.to_string(),
        native_symbol: spec.3.to_string(),
    }
}

fn custom_chain_entry(chain: &ConfiguredChain) -> ChainEntry {
    ChainEntry {
        aliases: chain
            .aliases
            .iter()
            .map(|alias| canonicalize(alias))
            .collect(),
        chain_id: chain.chain_id,
        display_name: chain.name.clone(),
        is_builtin: false,
        key: canonicalize(&chain.name),
        native_symbol: chain.native_symbol.clone(),
    }
}

fn canonicalize(value: &str) -> String {
    value
        .trim()
        .replace('_', " ")
        .split_whitespace()
        .map(|segment| segment.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}
