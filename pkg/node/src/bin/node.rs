// lint-long-file-override allow-max-lines=300
use std::collections::HashMap;
use std::sync::Arc;
use std::{pin::Pin, time::Duration};

use clap::Parser;
use contextful::{Contextful, ResultContextExt};
use futures::{
    Future,
    future::{FutureExt, pending},
    stream::{FuturesUnordered, StreamExt},
};
use node::{Mode, Node, TracingInitError, TxnStats};
use node::{
    config::{Config, cli::CliArgs},
    create_rpc_server,
};
use rpc::tracing::setup_tracing;
use zk_circuits::BbBackend;

/// Run the contract worker with restart attempts on failure.
async fn run_contract_worker_with_retries(
    contract: contracts::RollupContract,
    interval: Duration,
    max_restarts: u32,
    delay_on_error: Duration,
) -> contracts::Result<()> {
    let mut attempts: u32 = 0;
    let reset_duration = Duration::from_secs(30 * 60); // 30 minutes

    loop {
        let start_time = std::time::Instant::now();
        match contract.worker(interval).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let ran_for = start_time.elapsed();
                if ran_for >= reset_duration {
                    tracing::info!(
                        ran_for = ?ran_for,
                        old_attempts = attempts,
                        "contract worker ran long enough; resetting attempt counter",
                    );
                    attempts = 0;
                }

                attempts += 1;

                if attempts > max_restarts {
                    tracing::error!(
                        attempts,
                        max_restarts,
                        error = ?e,
                        "contract worker exceeded restart attempts",
                    );
                    return Err(e);
                }

                tracing::warn!(
                    attempts,
                    max_restarts,
                    ran_for = ?ran_for,
                    error = ?e,
                    "contract worker failed; restarting",
                );

                tokio::time::sleep(delay_on_error).await;
            }
        }
    }
}

#[allow(clippy::result_large_err)]
#[tokio::main]
async fn main() -> node::Result<()> {
    let args = CliArgs::parse();

    let config = Config::from_env(args.clone())?;

    let _guard = match setup_tracing(
        &[
            "node",
            "solid",
            "smirk",
            "p2p2",
            "prover",
            "zk_primitives",
            "contracts",
            "block_store",
            "notes",
        ],
        &args.log_level,
        &args.log_format,
        config.sentry_dsn.clone(),
        config.env_name.clone(),
    )
    .context("initialize tracing")
    {
        Ok(guard) => guard,
        Err(err) => {
            let err = err.map_source(|source| {
                let boxed: Box<dyn std::error::Error + Send + Sync> = source.into();
                TracingInitError::from(boxed)
            });
            return Err(err.into());
        }
    };

    // Listen address of the server
    let rpc_laddr = config.rpc_laddr.clone();

    // Private key
    let peer_signer = config.secret_key.clone();

    // web3 depends on a different version ofsecp256k1, which carries an error type that
    // does not implement our `From<Contextful<secp256k1::Error>>`; map it into
    // a generic InvalidSecretKey instead of using `?`.
    let secret_key =
        web3::signing::SecretKey::from_slice(&config.secret_key.secret_key().secret_bytes()[..])
            .map_err(|_err| {
                node::Error::Secp256k1(Contextful::new(
                    "load secret key",
                    secp256k1::Error::InvalidSecretKey,
                ))
            })?;
    let mut rollup_contracts_map = HashMap::new();
    for chain in &config.chains {
        let client = contracts::Client::new(&chain.eth_rpc_url, config.minimum_gas_price_gwei);
        let contract = contracts::RollupContract::load(
            client,
            u128::from(chain.chain_id),
            &chain.rollup_contract_addr,
            secret_key,
        )
        .await
        .context(format!("load rollup contract for chain {}", chain.chain_id))?;
        rollup_contracts_map.insert(chain.chain_id, contract);
    }

    if rollup_contracts_map.is_empty() {
        return Err(node::Error::MissingPrimaryChainConfig);
    }

    let rollup_contracts = Arc::new(rollup_contracts_map);

    // Services
    let bb_backend: Arc<dyn BbBackend> = Arc::new(barretenberg_cli::CliBackend);
    let node = Node::new(
        peer_signer,
        Arc::clone(&rollup_contracts),
        config.clone(),
        Arc::clone(&bb_backend),
    )?;
    let mut contract_worker_tasks: Option<FuturesUnordered<_>> = {
        let tasks: FuturesUnordered<_> = rollup_contracts
            .values()
            .cloned()
            .map(|contract| {
                run_contract_worker_with_retries(
                    contract,
                    Duration::from_secs(30),
                    3,
                    Duration::from_secs(5),
                )
                .boxed()
            })
            .collect();

        if tasks.is_empty() { None } else { Some(tasks) }
    };
    let txn_stats = Arc::new(TxnStats::new(Arc::clone(&node.shared)));
    let server = create_rpc_server(
        &rpc_laddr,
        config.health_check_commit_interval_sec,
        Arc::clone(&node.shared),
        Arc::clone(&txn_stats),
    )
    .context("start RPC server")?;

    let prover_task: Pin<Box<dyn Future<Output = Result<(), node::prover::Error>>>> =
        if config.mode == Mode::Prover || config.mode == Mode::MockProver {
            Box::pin(node::prover::worker::run_prover(
                &config,
                Arc::clone(&node.shared),
            ))
        } else {
            Box::pin(async { futures::future::pending().await })
        };

    tokio::select! {
        res = node.run() => {
            tracing::info!("node shutdown: {:?}", res);
        }
        res = prover_task => {
            tracing::info!("prover shutdown: {:?}", res);
        }
        res = server => {
            tracing::info!("rpc server shutdown: {:?}", res);
        }
        Some(res) = async {
            if let Some(tasks) = &mut contract_worker_tasks {
                tasks.next().await
            } else {
                pending::<Option<contracts::Result<()>>>().await
            }
        } => {
            match res {
                Ok(()) => tracing::info!("contract worker shutdown: Ok(())"),
                Err(e) => {
                    tracing::error!(error = ?e, "contract worker shutdown after retries");
                }
            }
        }
        res = txn_stats.clone().worker() => {
            tracing::info!("txn stats worker shutdown: {:?}", res);
        }
        res = txn_stats.today_stats_worker() => {
            tracing::info!("txn stats today worker shutdown: {:?}", res);
        }
    }

    Ok(())
}
