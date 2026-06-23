use serde_json::{Value, json};

use crate::{
    Error, Result,
    abi::parse_address,
    host::{self, DynamicContractScope},
};

const LEGACY_STORAGE_KEY: &str = "registry-config-v1";
const MAINNET_IDENTITY_REGISTRY: &str = "0x8004A169FB4a3325136EB29fA0ceB6D2e539a432";
const MAINNET_REPUTATION_REGISTRY: &str = "0x8004BAa17C55a88189AE136b182e5fdA19dE9b63";
const TESTNET_IDENTITY_REGISTRY: &str = "0x8004A818BFB912233c491871b3d84c89A494BD9e";
const TESTNET_REPUTATION_REGISTRY: &str = "0x8004B663056A597Dffe9eCcC1965A193B7388713";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryConfig {
    pub chain_id: u64,
    pub display_name: String,
    pub identity_registry: String,
    pub is_default_identity: bool,
    pub reputation_registry: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistrySelection {
    pub config: RegistryConfig,
    pub dynamic_contracts: Vec<DynamicContractScope>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
struct StoredConfig {
    #[serde(default)]
    chains: Vec<StoredChainConfig>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
struct StoredChainConfig {
    chain_id: u64,
    identity_registry: String,
    reputation_registry: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Deployment {
    chain_id: u64,
    display_name: &'static str,
    identity_registry: &'static str,
    reputation_registry: &'static str,
}

pub fn select(
    chain_id: u64,
    chain_display_name: &str,
    override_identity: Option<&str>,
) -> Result<RegistrySelection> {
    let stored_chain = load_stored_chain_config(chain_id)?;
    let default = deployment_for_chain_id(chain_id);
    let identity_registry = override_identity
        .map(normalize_address)
        .transpose()?
        .or_else(|| {
            stored_chain
                .as_ref()
                .map(|chain| chain.identity_registry.clone())
        })
        .or_else(|| {
            default
                .as_ref()
                .map(|deployment| deployment.identity_registry.to_string())
        })
        .ok_or(Error::UnsupportedChain { chain_id })?;
    let reputation_registry = stored_chain
        .as_ref()
        .map(|chain| chain.reputation_registry.clone())
        .or_else(|| {
            default
                .as_ref()
                .map(|deployment| deployment.reputation_registry.to_string())
        })
        .unwrap_or_else(|| TESTNET_REPUTATION_REGISTRY.to_string());
    let default_identity = default
        .as_ref()
        .map(|deployment| deployment.identity_registry)
        .unwrap_or_default();
    let is_default_identity = identity_registry.eq_ignore_ascii_case(default_identity);
    let dynamic_contracts = (!is_default_identity)
        .then(|| DynamicContractScope {
            chain: chain_display_name.to_string(),
            contract: identity_registry.clone(),
            reason: "ERC-8004 identity registry override".to_string(),
        })
        .into_iter()
        .collect();

    Ok(RegistrySelection {
        config: RegistryConfig {
            chain_id,
            display_name: default
                .as_ref()
                .map(|deployment| deployment.display_name.to_string())
                .unwrap_or_else(|| chain_display_name.to_string()),
            identity_registry,
            is_default_identity,
            reputation_registry,
        },
        dynamic_contracts,
    })
}

pub fn show(chain_id: u64, chain_display_name: &str) -> Result<RegistryConfig> {
    Ok(select(chain_id, chain_display_name, None)?.config)
}

pub fn set(
    chain_id: u64,
    chain_display_name: &str,
    identity_registry: &str,
    reputation_registry: Option<&str>,
) -> Result<RegistryConfig> {
    let identity_registry = normalize_address(identity_registry)?;
    let default = deployment_for_chain_id(chain_id);
    let reputation_registry = reputation_registry
        .map(normalize_address)
        .transpose()?
        .unwrap_or_else(|| {
            default
                .clone()
                .map(|deployment| deployment.reputation_registry.to_string())
                .unwrap_or_else(|| TESTNET_REPUTATION_REGISTRY.to_string())
        });
    let default_identity = default
        .as_ref()
        .map(|deployment| deployment.identity_registry)
        .unwrap_or_default();
    let is_default_identity = identity_registry.eq_ignore_ascii_case(default_identity);
    let display_name = default
        .as_ref()
        .map(|deployment| deployment.display_name.to_string())
        .unwrap_or_else(|| chain_display_name.to_string());
    let stored = StoredChainConfig {
        chain_id,
        identity_registry: identity_registry.clone(),
        reputation_registry: reputation_registry.clone(),
    };
    let stored = serde_json::to_string(&stored).map_err(|err| Error::Serialization {
        reason: err.to_string(),
    })?;
    host::storage_set(&chain_storage_key(chain_id), &stored)?;
    Ok(RegistryConfig {
        chain_id,
        display_name,
        identity_registry,
        is_default_identity,
        reputation_registry,
    })
}

pub fn to_json(config: &RegistryConfig) -> Value {
    json!({
        "chain_id": config.chain_id,
        "display_name": config.display_name,
        "identity_registry": config.identity_registry,
        "identity_registry_source": if config.is_default_identity { "default" } else { "override" },
        "reputation_registry": config.reputation_registry,
    })
}

fn load_stored_chain_config(chain_id: u64) -> Result<Option<StoredChainConfig>> {
    if let Some(value) = host::storage_get(&chain_storage_key(chain_id))? {
        return Ok(Some(parse_storage_value(value)?));
    }

    let Some(value) = host::storage_get(LEGACY_STORAGE_KEY)? else {
        return Ok(None);
    };
    Ok(parse_storage_value::<StoredConfig>(value)?
        .chains
        .into_iter()
        .find(|chain| chain.chain_id == chain_id))
}

fn parse_storage_value<T: serde::de::DeserializeOwned>(value: Value) -> Result<T> {
    match value {
        Value::String(value) => {
            serde_json::from_str::<T>(&value).map_err(|err| Error::Serialization {
                reason: err.to_string(),
            })
        }
        value => serde_json::from_value::<T>(value).map_err(|err| Error::Serialization {
            reason: err.to_string(),
        }),
    }
}

fn chain_storage_key(chain_id: u64) -> String {
    format!("registry-config-v1-{chain_id}")
}

fn normalize_address(value: &str) -> Result<String> {
    Ok(format!("{:#x}", parse_address(value)?))
}

fn deployment_for_chain_id(chain_id: u64) -> Option<Deployment> {
    DEPLOYMENTS
        .iter()
        .find(|deployment| deployment.chain_id == chain_id)
        .cloned()
}

const fn mainnet(chain_id: u64, display_name: &'static str) -> Deployment {
    Deployment {
        chain_id,
        display_name,
        identity_registry: MAINNET_IDENTITY_REGISTRY,
        reputation_registry: MAINNET_REPUTATION_REGISTRY,
    }
}

const fn testnet(chain_id: u64, display_name: &'static str) -> Deployment {
    Deployment {
        chain_id,
        display_name,
        identity_registry: TESTNET_IDENTITY_REGISTRY,
        reputation_registry: TESTNET_REPUTATION_REGISTRY,
    }
}

const DEPLOYMENTS: &[Deployment] = &[
    mainnet(1, "Ethereum Mainnet"),
    testnet(11155111, "Ethereum Sepolia"),
    mainnet(8453, "Base Mainnet"),
    testnet(84532, "Base Sepolia"),
    mainnet(2741, "Abstract Mainnet"),
    testnet(11124, "Abstract Testnet"),
    mainnet(42161, "Arbitrum Mainnet"),
    testnet(421614, "Arbitrum Testnet"),
    mainnet(43114, "Avalanche Mainnet"),
    testnet(43113, "Avalanche Testnet"),
    mainnet(56, "BSC Mainnet"),
    testnet(97, "BSC Testnet"),
    mainnet(42220, "Celo Mainnet"),
    testnet(11142220, "Celo Testnet"),
    mainnet(100, "Gnosis Mainnet"),
    mainnet(2345, "GOAT Network Mainnet"),
    mainnet(59144, "Linea Mainnet"),
    testnet(59141, "Linea Sepolia"),
    mainnet(5000, "Mantle Mainnet"),
    testnet(5003, "Mantle Testnet"),
    mainnet(4326, "MegaETH Mainnet"),
    testnet(6343, "MegaETH Testnet"),
    mainnet(1088, "Metis Mainnet"),
    testnet(59902, "Metis Sepolia"),
    mainnet(143, "Monad Mainnet"),
    testnet(10143, "Monad Testnet"),
    mainnet(10, "Optimism Mainnet"),
    testnet(11155420, "Optimism Testnet"),
    mainnet(137, "Polygon Mainnet"),
    testnet(80002, "Polygon Amoy"),
    mainnet(534352, "Scroll Mainnet"),
    testnet(534351, "Scroll Testnet"),
    mainnet(1187947933, "SKALE Base Mainnet"),
    testnet(324705682, "SKALE Base Sepolia"),
    mainnet(1868, "Soneium Mainnet"),
    testnet(1946, "Soneium Minato"),
    mainnet(167000, "Taiko Mainnet"),
    testnet(167012, "Taiko Hoodi"),
    mainnet(196, "XLayer Mainnet"),
    testnet(1952, "XLayer Testnet"),
    testnet(296, "Hedera Testnet"),
    testnet(5042002, "Arc Testnet"),
    mainnet(45056, "Billions Mainnet"),
    testnet(6913, "Billions Testnet"),
    mainnet(1776, "Injective Mainnet"),
    testnet(1439, "Injective Testnet"),
];
