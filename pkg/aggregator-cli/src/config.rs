use clap::Parser;
use element::Element;
use rpc::tracing::{LogFormat, LogLevel};
use std::{collections::BTreeMap, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootHeightOverride {
    pub(crate) root_hash: Element,
    pub(crate) height: u64,
}

pub(crate) fn parse_root_height_override(
    raw: &str,
) -> std::result::Result<RootHeightOverride, String> {
    let Some((raw_root_hash, raw_height)) = raw.split_once('=') else {
        return Err(
            "invalid override format, expected ROOT_HASH=HEIGHT (e.g. 0xabc123=42)".to_owned(),
        );
    };

    let root_hash = Element::from_str(raw_root_hash.trim())
        .map_err(|_| format!("invalid root hash `{}` in override", raw_root_hash.trim()))?;
    let height = raw_height
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("invalid height `{}` in override", raw_height.trim()))?;

    Ok(RootHeightOverride { root_hash, height })
}

pub(crate) fn select_rollup_tree_seed_height(
    contract_height: u64,
    contract_root: Element,
    overrides: &[RootHeightOverride],
) -> u64 {
    overrides
        .iter()
        .find_map(|entry| (entry.root_hash == contract_root).then_some(entry.height))
        .unwrap_or(contract_height)
}

pub(crate) fn validate_rollup_root_height_overrides(
    overrides: &[RootHeightOverride],
) -> std::result::Result<(), String> {
    let mut heights_by_root = BTreeMap::<Element, u64>::new();

    for override_entry in overrides {
        if let Some(existing_height) = heights_by_root.get(&override_entry.root_hash)
            && *existing_height != override_entry.height
        {
            return Err(format!(
                "conflicting heights for root hash 0x{}: {} and {}",
                override_entry.root_hash.to_hex(),
                existing_height,
                override_entry.height
            ));
        }

        heights_by_root.insert(override_entry.root_hash, override_entry.height);
    }

    Ok(())
}

#[derive(Debug, Parser)]
#[command(author, version, about = "Polybase Aggregator CLI", long_about = None)]
pub struct Config {
    /// Node RPC URL
    #[arg(long, env = "NODE_RPC_URL", default_value = "http://localhost:8080")]
    pub node_rpc_url: String,

    /// Log level
    #[arg(long, env = "LOG_LEVEL", default_value = "INFO")]
    pub log_level: LogLevel,

    /// Log format
    #[arg(long, env = "LOG_FORMAT", default_value = "PRETTY")]
    pub log_format: LogFormat,

    /// Environment name
    #[arg(long, env = "ENV_NAME", default_value = "dev")]
    pub env_name: String,

    /// Ethereum RPC URL used to interact with the rollup contract
    #[arg(long, env = "EVM_RPC_URL", default_value = "http://localhost:8545")]
    pub evm_rpc_url: String,

    /// Hex-encoded rollup contract address
    #[arg(
        long,
        env = "ROLLUP_CONTRACT_ADDRESS",
        default_value = "0xdc64a140aa3e981100a9beca4e685f962f0cf6c9"
    )]
    pub rollup_contract_address: String,

    /// Hex-encoded (0x-prefixed) secret key for submitting transactions
    #[arg(
        long,
        env = "EVM_SECRET_KEY",
        default_value = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    )]
    pub evm_secret_key: String,

    /// Ethereum chain id for the rollup contract
    #[arg(long, env = "CHAIN_ID", default_value_t = 1337)]
    pub chain_id: u64,

    /// Optional minimum gas price (in gwei) to use when submitting rollup proofs
    #[arg(long, env = "MINIMUM_GAS_PRICE_GWEI")]
    pub minimum_gas_price_gwei: Option<u64>,

    /// Poll interval (milliseconds) between aggregation attempts when running continuously
    #[arg(long, env = "POLL_INTERVAL_MS", default_value_t = 5_000)]
    pub poll_interval_ms: u64,

    /// Number of blocks to aggregate per step (must be a power of two)
    #[arg(long, env = "BLOCK_BATCH_SIZE", default_value_t = 2)]
    pub block_batch_size: usize,

    /// Gas to spend per burn call when submitting rollups
    #[arg(long, env = "GAS_PER_BURN_CALL", default_value_t = 1_000_000)]
    pub gas_per_burn_call: u128,

    /// Rollup transaction receipt timeout in milliseconds
    #[arg(long, env = "ROLLUP_RECEIPT_TIMEOUT_MS", default_value_t = 300_000)]
    pub rollup_receipt_timeout_ms: u64,

    /// Poll interval in milliseconds while waiting for rollup receipt
    #[arg(long, env = "ROLLUP_RECEIPT_POLL_INTERVAL_MS", default_value_t = 1_000)]
    pub rollup_receipt_poll_interval_ms: u64,

    /// Number of times to retry rollup submission on failure
    #[arg(long, env = "ROLLUP_SUBMIT_RETRY_ATTEMPTS", default_value_t = 3)]
    pub rollup_submit_retry_attempts: usize,

    /// Delay in milliseconds between rollup submission retries
    #[arg(long, env = "ROLLUP_SUBMIT_RETRY_DELAY_MS", default_value_t = 1_000)]
    pub rollup_submit_retry_delay_ms: u64,

    /// When set, execute a single aggregation step and exit
    #[arg(long, env = "RUN_ONCE", default_value_t = false)]
    pub run_once: bool,

    /// Optional barretenberg API server base URL; when provided aggregator-cli uses it instead of bb CLI
    #[arg(long, env = "BARRETENBERG_API_URL")]
    pub barretenberg_api_url: Option<String>,

    /// Maximum number of concurrent barretenberg proofs/verifications (defaults to 1 to reduce RAM usage)
    #[arg(long, env = "BB_MAX_CONCURRENCY", default_value_t = 1)]
    pub bb_max_concurrency: usize,

    /// Barretenberg API request timeout in milliseconds
    #[arg(long, env = "BB_TIMEOUT_MS", default_value_t = 300_000)]
    pub bb_timeout_ms: u64,

    /// Barretenberg API TCP connect timeout in milliseconds
    #[arg(long, env = "BB_CONNECT_TIMEOUT_MS", default_value_t = 1_000)]
    pub bb_connect_timeout_ms: u64,

    /// Barretenberg API permit acquisition timeout in milliseconds
    #[arg(long, env = "BB_PERMIT_TIMEOUT_MS", default_value_t = 100)]
    pub bb_permit_timeout_ms: u64,

    /// Buffer time in milliseconds to wait for 100-continue response from Barretenberg API
    #[arg(
        long,
        env = "BB_EXPECT_CONTINUE_TIMEOUT_BUFFER_MS",
        default_value_t = 500
    )]
    pub bb_expect_continue_timeout_buffer_ms: u64,

    /// Delay in milliseconds between Barretenberg API retries
    #[arg(long, env = "BB_RETRY_DELAY_MS", default_value_t = 500)]
    pub bb_retry_delay_ms: u64,

    /// Maximum duration in milliseconds to retry Barretenberg API requests
    #[arg(long, env = "BB_MAX_RETRY_DURATION_MS", default_value_t = 3_600_000)]
    pub bb_max_retry_duration_ms: u64,

    /// Number of steps to pipeline (prepare next step while proving current step)
    #[arg(long, env = "PIPELINE_DEPTH", default_value_t = 1)]
    pub pipeline_depth: usize,

    /// Optional root-hash keyed overrides for rollup tree seed height.
    ///
    /// Format per entry: `ROOT_HASH=HEIGHT`, for example `0xabc123=42`.
    /// Repeat the flag or provide a comma-separated list.
    #[arg(
        long,
        env = "ROLLUP_ROOT_HEIGHT_OVERRIDES",
        value_delimiter = ',',
        value_name = "ROOT_HASH=HEIGHT",
        value_parser = parse_root_height_override
    )]
    pub rollup_root_height_overrides: Vec<RootHeightOverride>,
}
