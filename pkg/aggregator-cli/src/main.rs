// lint-long-file-override allow-max-lines=300
mod config;
mod error;
mod runner;

#[cfg(test)]
mod tests;

use std::{str::FromStr, sync::Arc, time::Duration};

use aggregator::{
    AggAggCircuitInterface, AggFinalCircuitInterface, AggUtxoCircuitInterface, Aggregator,
    BlockProver, ContractsRollupContract, LimitedBbBackend, RetryableRollupContract,
};
use aggregator_interface::{BlockProver as BlockProverTrait, PrioritizableBbBackend};
use barretenberg_api_client::ClientBackend;
use barretenberg_cli::CliBackend;
use barretenberg_interface::BbBackend;
use clap::Parser;
use contextful::ResultContextExt;
use contracts::{Client, SecretKey};
use element::Element;
use node_client_http::{NodeClientHttp, Url as NodeUrl};
use node_interface::NodeClient;
use rpc::tracing::setup_tracing;
use tokio::signal;
use url::Url as ApiUrl;
use zk_circuits::{AggAggCircuit, AggFinalCircuit, AggUtxoCircuit};

use crate::config::{
    Config, select_rollup_tree_seed_height, validate_rollup_root_height_overrides,
};
use crate::error::{Error, Result};
use crate::runner::{build_rollup_tree, run_loop, run_once};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();
    let _guard = setup_tracing(
        &[
            "aggregator",
            "aggregator_cli",
            "aggregator_interface",
            "notes",
        ],
        &config.log_level,
        &config.log_format,
        None,
        config.env_name.clone(),
    )
    .map_err(|err| Error::InvalidConfig(format!("failed to setup tracing: {err}")))?;

    if config.bb_max_concurrency == 0 {
        return Err(Error::InvalidConfig(
            "BB_MAX_CONCURRENCY must be greater than zero".to_string(),
        ));
    }
    if config.pipeline_depth == 0 {
        return Err(Error::InvalidConfig(
            "PIPELINE_DEPTH must be greater than zero".to_string(),
        ));
    }
    validate_rollup_root_height_overrides(&config.rollup_root_height_overrides)
        .map_err(Error::InvalidConfig)?;

    let secret_key = parse_secret_key(&config.evm_secret_key)?;
    let client =
        Client::new(&config.evm_rpc_url, config.minimum_gas_price_gwei).with_latest_nonce();
    let rollup_contract = Arc::new(
        contracts::RollupContract::load(
            client,
            config.chain_id as u128,
            &config.rollup_contract_address,
            secret_key,
        )
        .await
        .context("load rollup contract")?,
    );

    let node_url = NodeUrl::parse(&config.node_rpc_url).context("parse node rpc url")?;
    let http_client = NodeClientHttp::new(node_url);

    let contract_height = rollup_contract
        .block_height()
        .await
        .context("fetch rolled block height")?;
    let rolled_root = Element::from_be_bytes(
        rollup_contract
            .root_hash()
            .await
            .context("fetch rolled root hash")?
            .to_fixed_bytes(),
    );
    let rolled_height = select_rollup_tree_seed_height(
        contract_height,
        rolled_root,
        &config.rollup_root_height_overrides,
    );
    if rolled_height != contract_height {
        tracing::info!(
            contract_height,
            rolled_height,
            contract_root = ?rolled_root,
            "using configured rollup tree height override for contract root hash"
        );
    }
    let rollup_tree = build_rollup_tree(&http_client, rolled_height, rolled_root).await?;

    let node_client: Arc<dyn NodeClient> = Arc::new(http_client.clone());
    let bb_backend = build_backend(&config)?;
    let agg_utxo_circuit: Arc<dyn AggUtxoCircuitInterface> = Arc::new(AggUtxoCircuit);
    let agg_agg_circuit: Arc<dyn AggAggCircuitInterface> = Arc::new(AggAggCircuit);
    let agg_final_circuit: Arc<dyn AggFinalCircuitInterface> = Arc::new(AggFinalCircuit);
    let block_prover: Arc<dyn BlockProverTrait> = Arc::new(BlockProver::new(
        Arc::clone(&node_client),
        Arc::clone(&agg_utxo_circuit),
        Arc::clone(&agg_agg_circuit),
    ));
    let base_contract_adapter = ContractsRollupContract::new(
        Arc::clone(&rollup_contract),
        Duration::from_millis(config.rollup_receipt_timeout_ms),
        Duration::from_millis(config.rollup_receipt_poll_interval_ms),
    );
    let contract_adapter = Arc::new(RetryableRollupContract::new(
        Box::new(base_contract_adapter),
        config.rollup_submit_retry_attempts,
        Duration::from_millis(config.rollup_submit_retry_delay_ms),
    ));

    if config.block_batch_size < 2 {
        return Err(Error::InvalidConfig(
            "BLOCK_BATCH_SIZE must be at least 2".to_string(),
        ));
    }
    if !config.block_batch_size.is_power_of_two() {
        return Err(Error::InvalidConfig(
            "BLOCK_BATCH_SIZE must be a power of two".to_string(),
        ));
    }
    if config.gas_per_burn_call == 0 {
        return Err(Error::InvalidConfig(
            "GAS_PER_BURN_CALL must be greater than zero".to_string(),
        ));
    }

    let aggregator = Arc::new(Aggregator::new(
        node_client,
        contract_adapter,
        block_prover,
        Box::new(rollup_tree),
        config.block_batch_size,
        config.gas_per_burn_call,
        Arc::clone(&agg_agg_circuit),
        agg_final_circuit,
    ));

    if config.run_once {
        run_once(aggregator, bb_backend, config.pipeline_depth).await?;
        return Ok(());
    }

    let shutdown = Box::pin(async {
        let _ = signal::ctrl_c().await;
    });

    run_loop(
        aggregator,
        bb_backend,
        Duration::from_millis(config.poll_interval_ms),
        config.pipeline_depth,
        shutdown,
    )
    .await
}

fn build_backend(config: &Config) -> Result<Arc<dyn PrioritizableBbBackend>> {
    let base_backend: Arc<dyn BbBackend> = if let Some(ref raw_url) = config.barretenberg_api_url {
        let api_url = ApiUrl::parse(raw_url).context("parse barretenberg api url")?;
        let client = ClientBackend::with_retry_policy(
            api_url,
            Duration::from_millis(config.bb_timeout_ms),
            Duration::from_millis(config.bb_connect_timeout_ms),
            Some(Duration::from_millis(config.bb_permit_timeout_ms)),
            Duration::from_millis(config.bb_retry_delay_ms),
            Duration::from_millis(config.bb_max_retry_duration_ms),
            Duration::from_millis(config.bb_expect_continue_timeout_buffer_ms),
        )
        .context("create barretenberg api client")?;
        Arc::new(client)
    } else {
        Arc::new(CliBackend)
    };

    Ok(Arc::new(LimitedBbBackend::new(
        base_backend,
        config.bb_max_concurrency,
    )))
}

fn parse_secret_key(raw: &str) -> Result<SecretKey> {
    let trimmed = raw.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    SecretKey::from_str(hex).map_err(|err| Error::InvalidSecretKey(err.to_string()))
}
