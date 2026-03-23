use std::time::Duration;

use burn_substitutor::Error;
use clap::Parser;
use contextful::prelude::*;
use contracts::{Address, ConfirmationType, RollupContract, U256};
use node_client_http::{NodeClientHttp, Url};
use rpc::tracing::{LogFormat, LogLevel};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[clap(name = "Polybase Burn Subsitutor")]
#[command(author = "Polybase <hello@polybase.xyz>")]
#[command(author, version, about = "Polybase Burn Subsitutor - enables instant withdrawals", long_about = None)]
#[command(propagate_version = true)]
pub struct Config {
    #[arg(value_enum, long, env = "LOG_LEVEL", default_value = "INFO")]
    log_level: LogLevel,

    #[arg(value_enum, long, env = "LOG_FORMAT", default_value = "PRETTY")]
    log_format: LogFormat,

    #[arg(
        long,
        env = "ROLLUP_CONTRACT_ADDRESS",
        default_value = "0xdc64a140aa3e981100a9beca4e685f962f0cf6c9"
    )]
    rollup_contract_address: String,

    #[arg(
        long,
        env = "USDC_CONTRACT_ADDRESS",
        default_value = "0x5fbdb2315678afecb367f032d93f642f64180aa3"
    )]
    usdc_contract_address: String,

    #[arg(
        long,
        env = "EVM_SECRET_KEY",
        default_value = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    )]
    evm_secret_key: String,

    #[arg(long, env = "EVM_RPC_URL", default_value = "http://localhost:8545")]
    evm_rpc_url: String,

    /// Ethereum chain id for the rollup contract
    #[arg(long, env = "CHAIN_ID", default_value_t = 137)]
    chain_id: u64,

    #[arg(long, env = "NODE_RPC_URL", default_value = "http://localhost:8080/v0")]
    node_rpc_url: String,

    #[arg(long, env = "MINIMUM_GAS_PRICE_GWEI")]
    minimum_gas_price_gwei: Option<u64>,

    #[arg(long, env = "EXCLUDED_BURN_ADDRESSES", value_delimiter = ',')]
    excluded_burn_addresses: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = Config::parse();

    let excluded_burn_addresses = parse_excluded_addresses(&config.excluded_burn_addresses)?;

    rpc::tracing::setup_tracing(
        &[
            "burn_substitutor",
            "node",
            "solid",
            "smirk",
            "p2p",
            "prover",
            "zk_primitives",
            "contracts",
            "block_store",
            "element",
            "notes",
        ],
        &config.log_level,
        &config.log_format,
        std::env::var("SENTRY_DSN").ok(),
        std::env::var("ENV_NAME").unwrap_or_else(|_| "dev".to_owned()),
    )
    .map_err(std::io::Error::other)
    .context("setup tracing")?;

    let wallet = contracts::wallet::Wallet::new_from_str(
        config
            .evm_secret_key
            .strip_prefix("0x")
            .ok_or(Error::MissingField { name: "0x prefix" })?,
    )
    .context("parse secret key")?;
    let secret_key = wallet.web3_secret_key();

    let client = contracts::Client::new(&config.evm_rpc_url, config.minimum_gas_price_gwei);
    let rpc_chain_id = client
        .chain_id_contracts()
        .await
        .context("fetch rpc chain id")?;
    let config_chain_id = U256::from(config.chain_id);
    if rpc_chain_id != config_chain_id {
        return Err(Error::ChainIdMismatch {
            config_chain_id: config.chain_id,
            rpc_chain_id,
        });
    }
    let primary_chain_id = u128::from(config.chain_id);
    let rollup_contract = RollupContract::load(
        client.clone(),
        primary_chain_id,
        &config.rollup_contract_address,
        secret_key,
    )
    .await
    .context("load rollup contract")?;
    let usdc_contract = contracts::USDCContract::load(
        client.clone(),
        primary_chain_id,
        &config.usdc_contract_address,
        secret_key,
    )
    .await
    .context("load usdc contract")?;

    let node_client =
        NodeClientHttp::new(Url::parse(&config.node_rpc_url).context("parse node rpc url")?);

    if usdc_contract
        .allowance(rollup_contract.signer_address, rollup_contract.address())
        .await
        .context("fetch allowance")?
        != U256::MAX
    {
        let approve_txn = usdc_contract
            .approve_max(rollup_contract.address())
            .await
            .context("approve max")?;
        client
            .wait_for_confirm(
                approve_txn,
                Duration::from_secs(1),
                ConfirmationType::Latest,
            )
            .await
            .context("wait for approve confirm")?;
    }

    let mut substitutor = burn_substitutor::BurnSubstitutor::new(
        rollup_contract,
        usdc_contract,
        Box::new(node_client),
        Duration::from_secs(1),
        excluded_burn_addresses,
    );

    tracing::info!("Starting burn substitutor");

    loop {
        let substitutions = substitutor.tick().await?;
        for nullifier in &substitutions {
            tracing::info!(?nullifier, "Substituted burn");
        }

        if substitutions.is_empty() {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

fn parse_excluded_addresses(addresses: &[String]) -> Result<Vec<Address>, Error> {
    addresses
        .iter()
        .map(|address| {
            address
                .parse::<Address>()
                .map_err(|_| Error::InvalidAddress {
                    address: address.to_owned(),
                })
        })
        .collect()
}
