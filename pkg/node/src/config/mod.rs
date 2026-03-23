// lint-long-file-override allow-max-lines=400
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

use self::cli::CliArgs;
use crate::{Error, Mode, Result};
use contextful::ResultContextExt;
use dirs::home_dir;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use primitives::peer::PeerIdSigner;
use serde::{Deserialize, Deserializer, de::Error as DeError};

pub mod cli;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ChainConfig {
    pub chain_id: u64,
    pub eth_rpc_url: String,
    pub rollup_contract_addr: String,
    #[serde(default)]
    pub safe_eth_height_offset: u64,
    #[serde(default)]
    pub max_rollup_gas_price_gwei: Option<u64>,
}

// TODO: should we use kebab-case? Currently _ is used to split into
// multiple level dictionaries
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Sentry DSN URL
    pub sentry_dsn: Option<String>,

    /// Sentry environment name
    pub env_name: String,

    /// Maximum number of txns to include in a block
    pub block_txns_count: usize,

    /// Minimum block duration in seconds
    pub min_block_duration: usize,

    pub sync_chunk_size: u64,

    pub sync_timeout_ms: u64,

    pub fast_sync_threshold: u64,

    pub mode: Mode,

    /// Private key of validator
    pub secret_key: PeerIdSigner,

    /// RPC config
    pub rpc_laddr: String,

    /// P2P config
    pub p2p: ::p2p2::Config,

    /// Path to the database
    pub db_path: PathBuf,

    /// Path to Smirk
    pub smirk_path: PathBuf,

    /// Chain-specific configuration, accepting both TOML arrays and env var maps.
    #[serde(default, deserialize_with = "deserialize_chain_configs")]
    pub chains: Vec<ChainConfig>,

    #[serde(default)]
    pub eth_rpc_url: Option<String>,

    #[serde(default)]
    pub rollup_contract_addr: Option<String>,

    /// If the last commit is older than this, health check will fail
    pub health_check_commit_interval_sec: u64,

    pub rollup_wait_time_ms: u64,

    /// Optional postgres database for synchronization between provers
    pub prover_database_url: Option<String>,

    /// Blocks that should not be validated or rolled up
    pub bad_blocks: Vec<u64>,

    /// The minimum amount of gas (in gwei) to use for transactions
    pub minimum_gas_price_gwei: Option<u64>,

    /// Whether the prover worker should be started alongside the rollup worker
    #[serde(default = "default_true")]
    pub enable_prover_worker: bool,

    /// Whether the rollup worker should be started
    #[serde(default = "default_true")]
    pub enable_rollup_worker: bool,

    #[serde(default, rename = "safe-eth-height-offset")]
    legacy_safe_eth_height_offset: Option<u64>,
}

impl Config {
    /// The text of the default config string
    pub const DEFAULT_STR: &str = include_str!("./default_config.toml");

    /// Load a [`Config`] from a file and environment
    ///
    /// `config_path` doesn't need to point to an actual file
    pub fn from_env(args: CliArgs) -> Result<Self> {
        let cli_eth_rpc_url = args.eth_rpc_url.clone();
        let cli_rollup_contract_addr = args.rollup_contract_addr.clone();

        let mut config: Config = Figment::from(Toml::string(Self::DEFAULT_STR))
            .merge(Toml::file(args.config_path))
            .merge(
                Env::prefixed("POLY_")
                    .split("__")
                    .map(|k| k.as_str().replace('_', "-").into()),
            )
            .extract()
            .context("extract configuration")?;

        if let Some(mode) = args.mode {
            config.mode = mode;
        }

        if let Some(p2p_laddr) = args.p2p_laddr {
            config.p2p.laddr = p2p_laddr;
        }

        if let Some(p2p_dial) = args.p2p_dial {
            config.p2p.dial = p2p_dial;
        }

        if let Some(secret_key_path) = args.secret_key_path {
            let mut file = File::open(&secret_key_path).context(format!(
                "open secret key file {}",
                display_path(&secret_key_path)
            ))?;
            let mut key = String::new();
            file.read_to_string(&mut key).context(format!(
                "read secret key file {}",
                display_path(&secret_key_path)
            ))?;
            config.secret_key = PeerIdSigner::from_str(key.trim())
                .context("parse secret key from file contents")?;
        }

        if let Some(secret_key) = args.secret_key {
            config.secret_key = secret_key;
        }

        if let Some(rpc_laddr) = args.rpc_laddr {
            config.rpc_laddr = rpc_laddr;
        }

        if let Some(db_path) = args.db_path {
            config.db_path = db_path;
        }

        config.db_path = expand_home_if_needed(config.db_path)?;

        if let Some(smirk_path) = args.smirk_path {
            config.smirk_path = smirk_path;
        }

        config.smirk_path = expand_home_if_needed(config.smirk_path)?;

        if let Some(eth_rpc_url) = cli_eth_rpc_url {
            config.eth_rpc_url = Some(eth_rpc_url);
        }

        if let Some(rollup_contract_addr) = cli_rollup_contract_addr {
            config.rollup_contract_addr = Some(rollup_contract_addr);
        }

        if let Some(sync_chunk_size) = args.sync_chunk_size {
            config.sync_chunk_size = sync_chunk_size;
        }

        let legacy_safe_eth_height_offset = config.legacy_safe_eth_height_offset.take();
        let had_legacy_safe_eth_height_offset = legacy_safe_eth_height_offset.is_some();
        let legacy_safe_eth_height_offset_value = legacy_safe_eth_height_offset.unwrap_or_default();

        if config.chains.is_empty() {
            return Err(Error::MissingPrimaryChainConfig);
        }

        let single_chain = config.chains.len() == 1;

        if let Some(chain) = config.chains.first_mut() {
            if let Some(ref eth_rpc_url) = config.eth_rpc_url {
                chain.eth_rpc_url = eth_rpc_url.clone();
            } else {
                config.eth_rpc_url = Some(chain.eth_rpc_url.clone());
            }

            if let Some(ref rollup_contract_addr) = config.rollup_contract_addr {
                chain.rollup_contract_addr = rollup_contract_addr.clone();
            } else {
                config.rollup_contract_addr = Some(chain.rollup_contract_addr.clone());
            }

            if had_legacy_safe_eth_height_offset && single_chain {
                chain.safe_eth_height_offset = legacy_safe_eth_height_offset_value;
            }
        }

        Ok(config)
    }

    #[must_use]
    pub fn primary_chain(&self) -> Option<&ChainConfig> {
        self.chains.first()
    }

    #[must_use]
    pub fn chain_config(&self, chain_id: u64) -> Option<&ChainConfig> {
        self.chains.iter().find(|chain| chain.chain_id == chain_id)
    }

    #[must_use]
    pub fn primary_chain_id(&self) -> Option<u64> {
        self.primary_chain().map(|chain| chain.chain_id)
    }

    #[must_use]
    pub fn safe_eth_height_offset(&self, chain_id: u64) -> Option<u64> {
        self.chain_config(chain_id)
            .map(|chain| chain.safe_eth_height_offset)
    }

    #[must_use]
    pub fn primary_safe_eth_height_offset(&self) -> Option<u64> {
        self.primary_chain()
            .map(|chain| chain.safe_eth_height_offset)
    }
}

/// Allow `chains` to be described as either a TOML array or an env-var map produced
/// by Figment (from keys like `POLY_CHAINS__0__CHAIN_ID`), normalizing into a Vec.
fn deserialize_chain_configs<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<ChainConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Chains {
        Seq(Vec<ChainConfig>),
        Map(BTreeMap<String, ChainConfig>),
    }

    let Some(value) = Option::<Chains>::deserialize(deserializer)? else {
        return Ok(Vec::new());
    };

    match value {
        Chains::Seq(seq) => Ok(seq),
        Chains::Map(map) => {
            let mut entries = Vec::with_capacity(map.len());
            for (idx, chain) in map {
                let idx_num = idx.parse::<usize>().map_err(|_| {
                    DeError::custom(format!("chains index `{idx}` is not a number"))
                })?;
                entries.push((idx_num, chain));
            }
            entries.sort_by_key(|(idx, _)| *idx);
            Ok(entries.into_iter().map(|(_, chain)| chain).collect())
        }
    }
}

fn expand_home_if_needed(path: PathBuf) -> Result<PathBuf> {
    if !path.starts_with("~") {
        return Ok(path);
    }

    let Some(home) = home_dir() else {
        return Err(Error::ConfigMissingHomeDir { path });
    };

    let suffix = strip_tilde(&path);
    Ok(home.join(suffix))
}

fn strip_tilde(path: &Path) -> &Path {
    path.strip_prefix("~").unwrap_or(path)
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests;
