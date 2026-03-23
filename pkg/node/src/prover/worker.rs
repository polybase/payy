// lint-long-file-override allow-max-lines=800
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use super::{Error, Result};
use crate::config::Config;
use crate::constants::MERKLE_TREE_DEPTH;
use crate::errors::Error as NodeError;
use crate::prover::db::{LastSeenBlock, ProverDb};
use crate::types::BlockHeight;
use crate::{Mode, NodeShared, PersistentMerkleTree};
use contracts::{Error as ContractsError, RollupContract};
use either::Either;
use element::Element;
use futures::{StreamExt, future::pending};
use prover::{MAXIMUM_TXNS, RollupInput};
use prover::{Prover, Transaction};
use scopeguard::ScopeGuard;
use smirk::empty_tree_hash;
use tokio::sync::{Mutex, Notify, mpsc};
use tracing::{error, info, warn};
use web3::types::U256;

const GWEI_IN_WEI: u64 = 1_000_000_000;

fn gas_price_exceeds_threshold(max_rollup_gas_price_gwei: u64, current_gas_price: U256) -> bool {
    let max_gas_price = U256::from(max_rollup_gas_price_gwei) * U256::from(GWEI_IN_WEI);

    let exceeds_threshold = current_gas_price > max_gas_price;

    if exceeds_threshold {
        info!(
            ?current_gas_price,
            ?max_gas_price,
            "Skipping rollup due to high gas price"
        );
    }

    exceeds_threshold
}
use zk_primitives::{AggFinalProof, UtxoKind};

pub async fn run_prover(config: &Config, node: Arc<NodeShared>) -> Result<()> {
    let (client, postgres_future) = if let Some(url) = &config.prover_database_url {
        let connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let (client, conn) = tokio_postgres::Config::from_str(url)?
            .connect(postgres_native_tls::MakeTlsConnector::new(connector))
            .await?;
        let client = Arc::new(client);

        (Some(client), Either::Left(conn))
    } else {
        (
            None,
            Either::Right(futures::future::pending::<Result<(), _>>()),
        )
    };

    let secret_key =
        web3::signing::SecretKey::from_slice(&config.secret_key.secret_key().secret_bytes()[..])
            .unwrap();

    let chain = config
        .primary_chain()
        .ok_or_else(|| Error::Node(NodeError::UnsupportedChain { chain_id: 0 }))?;

    let contracts_client =
        contracts::Client::new(&chain.eth_rpc_url, config.minimum_gas_price_gwei);
    let contract = contracts::RollupContract::load(
        contracts_client,
        u128::from(chain.chain_id),
        &chain.rollup_contract_addr,
        secret_key,
    )
    .await?;

    let db_path = config.db_path.join("prover");
    let prover_state_db = Arc::new(ProverDb::create_or_load(&db_path)?);
    let prover = Arc::new(Prover::new(contract.clone(), Arc::clone(&node.bb_backend)));

    let smirk_path = config.smirk_path.join("prover");
    let notes_tree = Arc::new(Mutex::new(Some(PersistentMerkleTree::load(&smirk_path)?)));
    let delete_smirk = || {
        let notes_tree = Arc::clone(&notes_tree);
        let smirk_path = smirk_path.clone();
        move || async move {
            *notes_tree.lock().await = None;
            std::fs::remove_dir_all(&smirk_path)?;
            Ok(())
        }
    };
    let prover_worker_delete_smirk = delete_smirk();

    let proof_notifier = Arc::new(Notify::new());

    match prover_state_db.get_version()? {
        None => {
            let contract_height = contract.block_height().await?;
            let proofs =
                prover_state_db.list_rollups(BlockHeight(contract_height)..BlockHeight(u64::MAX));
            for proof in proofs {
                let (height, rollup_input) = proof?;
                if let Some(client) = client.as_ref() {
                    #[allow(clippy::disallowed_methods)]
                    client
                        .execute(
                            "INSERT INTO rollup_proofs (height, proof) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                            &[&(height.0 as i64), &borsh::to_vec(&rollup_input)?],
                        )
                        .await?;
                }
            }

            prover_state_db.set_version(1)?;
            info!("Migrated to version 1");
        }
        Some(1) => {}
        Some(n) => return Err(Error::InvalidProverVersion(n)),
    }

    let prover_worker_future = {
        let node = Arc::clone(&node);
        let contract_clone = contract.clone();
        let prover_state_db_clone = Arc::clone(&prover_state_db);
        let notes_tree_clone = Arc::clone(&notes_tree);
        let client_clone = client.clone();
        let prover_clone = Arc::clone(&prover);
        let proof_notifier_clone = Arc::clone(&proof_notifier);

        async move {
            if config.enable_prover_worker {
                run_prover_worker(
                    config,
                    node,
                    contract_clone,
                    prover_state_db_clone,
                    notes_tree_clone,
                    client_clone,
                    prover_worker_delete_smirk,
                    prover_clone,
                    proof_notifier_clone,
                )
                .await
            } else {
                info!("Prover worker disabled via configuration (enable_prover_worker = false)");
                pending::<Result<()>>().await
            }
        }
    };

    let rollup_worker_future = async {
        if config.enable_rollup_worker {
            run_rollup_worker(
                Duration::from_millis(config.rollup_wait_time_ms),
                contract,
                prover_state_db,
                prover,
                proof_notifier,
                chain.max_rollup_gas_price_gwei,
                None,
                client,
            )
            .await
        } else {
            info!("Rollup worker disabled via configuration (enable_rollup_worker = false)");
            pending::<Result<()>>().await
        }
    };

    tokio::try_join!(prover_worker_future, rollup_worker_future, async move {
        postgres_future.await?;
        Ok(())
    })?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_prover_worker<Fut>(
    config: &Config,
    node: Arc<NodeShared>,
    contract: RollupContract,
    prover_state_db: Arc<ProverDb>,
    notes_tree: Arc<Mutex<Option<PersistentMerkleTree>>>,
    postgres_db: Option<Arc<tokio_postgres::Client>>,
    delete_smirk: impl FnOnce() -> Fut,
    prover: Arc<Prover>,
    proof_notifier: Arc<Notify>,
) -> Result<()>
where
    Fut: Future<Output = Result<()>>,
{
    let initial_contract_block_height = BlockHeight(contract.block_height().await?);

    // Wait for node to notice it's out of sync
    tokio::time::sleep(Duration::from_secs(5)).await;
    // Wait for node to sync.
    // Trying to sync without waiting would mean invalid prover smirk tree,
    // if the node synced with fast sync.
    while node.is_out_of_sync() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let last_seen_block = prover_state_db
        .get_last_seen_block()?
        .unwrap_or(LastSeenBlock {
            height: BlockHeight(0),
            root_hash: empty_tree_hash(MERKLE_TREE_DEPTH),
        });

    if last_seen_block.root_hash != notes_tree.lock().await.as_ref().unwrap().tree().root_hash() {
        // We probably crashed after applying a block to the tree,
        // but before saving the last seen block
        tracing::error!(
            last_seen_block_root_hash = ?last_seen_block.root_hash,
            prover_root_hash = ?notes_tree.lock().await.as_ref().unwrap().tree().root_hash(),
            "The last seen block root hash does not match our tree's root hash"
        );

        prover_state_db.set_last_seen_block(LastSeenBlock {
            height: BlockHeight(0),
            root_hash: empty_tree_hash(MERKLE_TREE_DEPTH),
        })?;
        delete_smirk().await?;
        // Exit the process and let the supervisor restart us
        panic!(
            "Our tree's root hash does not match the last seen block's root hash. Please restart the prover"
        );
    }

    let height = last_seen_block.height + BlockHeight(1);
    let mut stream = node.commit_stream(Some(height)).await.peekable();

    while let Some(commit) = stream.next().await {
        let commit = commit?;
        let commit_height = commit.content.header.height;
        if commit.content.state.txns.is_empty() {
            prover_state_db.set_last_seen_block(LastSeenBlock {
                height: commit_height,
                root_hash: commit.content.state.root_hash,
            })?;
            continue;
        }

        let commit_was_already_rolled_up = initial_contract_block_height >= commit_height;
        let mut release_lock = Option::<ScopeGuard<(), _>>::None;
        let postgres_db_clone = postgres_db.clone();
        let we_are_the_prover_for_this_block = || async {
            let Some(postgres_db_clone) = postgres_db_clone else {
                return Ok(true);
            };

            let rows = postgres_db_clone
                .query(
                    "WITH height_if_no_proof AS (
                        SELECT CASE
                            WHEN EXISTS (SELECT 1 FROM rollup_proofs WHERE height = $1) THEN NULL
                            ELSE $1
                        END AS height
                    ) SELECT pg_try_advisory_lock((SELECT height::bigint FROM height_if_no_proof))",
                    &[&(commit_height.0 as i64)],
                )
                .await?;
            match rows.first().unwrap().get::<_, Option<bool>>(0) {
                // We acquired the lock
                Some(true) => {
                    // release it after the block is processed
                    release_lock.replace(scopeguard::guard((), |_| {
                        tokio::spawn(async move {
                            postgres_db_clone
                                .execute(
                                    "SELECT pg_advisory_unlock($1)",
                                    &[&(commit_height.0 as i64)],
                                )
                                .await
                                .unwrap();
                        });
                    }));

                    Ok::<bool, tokio_postgres::Error>(true)
                }
                // Someone else is proving this block
                Some(false) => Ok(false),
                // There already is a proof for this block
                None => Ok(false),
            }
        };
        let is_a_bad_block = config.bad_blocks.contains(&commit.content.header.height);
        if commit_was_already_rolled_up
            || is_a_bad_block
            || !we_are_the_prover_for_this_block().await?
        {
            NodeShared::apply_block_to_tree(
                notes_tree.lock().await.as_mut().unwrap(),
                &commit.content.state,
                commit.content.header.height,
            )?;
            prover_state_db.set_last_seen_block(LastSeenBlock {
                height: commit_height,
                root_hash: commit.content.state.root_hash,
            })?;
            continue;
        }

        tracing::info!(?commit, "Proving commit");
        tracing::info!(counter.proving_height = ?commit.content.header.height);
        let prover = Arc::clone(&prover);

        let mut txns = commit
            .content
            .state
            .txns
            .iter()
            .map(|utxo_proof| Ok(Some(Transaction::new(utxo_proof.clone()))))
            .collect::<Result<Vec<_>>>()?;

        while txns.len() < MAXIMUM_TXNS {
            txns.push(None);
        }

        let other_hash = *commit.content.header_hash().inner();

        let next_commit = Pin::new(&mut stream)
            .peek()
            .await
            .unwrap()
            .as_ref()
            .map_err(|_| Error::FailedToPeekNextCommit)?;

        let signatures = next_commit.content.header.approvals.clone();

        let mut notes_tree = notes_tree.lock().await;
        let notes_tree = notes_tree.as_mut().unwrap();

        let proof = match config.mode {
            Mode::MockProver => {
                let mut agg_final_proof = AggFinalProof::default();
                agg_final_proof.public_inputs.old_root = notes_tree.tree().root_hash();
                agg_final_proof.public_inputs.new_root = commit.content.state.root_hash;

                let mut messages = [Element::ZERO; 1000];
                let mut index = 0;

                for proof in commit.content.state.txns.iter() {
                    let proof_messages = match proof.kind() {
                        UtxoKind::Null | UtxoKind::Send => &[][..],
                        UtxoKind::Mint => &proof.public_inputs.messages[..4],
                        UtxoKind::Burn => &proof.public_inputs.messages[..],
                    };

                    for &message in proof_messages {
                        messages[index] = message;
                        index += 1;
                    }
                }

                agg_final_proof.public_inputs.messages = messages.to_vec();

                agg_final_proof
            }
            _ => {
                prover
                    .prove(notes_tree.tree(), commit_height.0, txns.try_into().unwrap())
                    .await?
            }
        };

        if proof.public_inputs.new_root != commit.content.state.root_hash {
            return Err(Error::RootMismatch {
                got: proof.public_inputs.new_root,
                expected: commit.content.state.root_hash,
            });
        }

        let rollup_input = RollupInput::new(proof, commit_height.0, other_hash, signatures);
        prover_state_db.set_rollup(commit_height, rollup_input.clone())?;

        NodeShared::apply_block_to_tree(notes_tree, &commit.content.state, commit_height)?;
        prover_state_db.set_last_seen_block(LastSeenBlock {
            height: commit_height,
            root_hash: commit.content.state.root_hash,
        })?;

        if let Some(postgres_db) = postgres_db.as_ref() {
            #[allow(clippy::disallowed_methods)]
            postgres_db
                .execute(
                    "INSERT INTO rollup_proofs (height, old_root, proof) VALUES ($1, $2, $3)",
                    &[
                        &(commit_height.0 as i64),
                        &rollup_input.old_root().to_be_bytes().to_vec(),
                        &borsh::to_vec(&rollup_input)?,
                    ],
                )
                .await
                .unwrap();
        }

        if commit.content.state.root_hash != notes_tree.tree().root_hash() {
            // Something went very wrong and our tree doesn't match the blockchain state
            return Err(Error::ProverTreeRootDoesNotMatchBlockStateRoot {
                prover_tree: notes_tree.tree().root_hash(),
                block_tree: commit.content.state.root_hash,
            });
        }

        proof_notifier.notify_waiters();
        tracing::info!(?commit, "Finished proving commit");
        tracing::info!(counter.proved_height = ?commit.content.header.height);
    }

    unreachable!()
}

#[allow(clippy::too_many_arguments)]
async fn run_rollup_worker(
    wait_time: Duration,
    mut rollup_contract: RollupContract,
    prover_state_db: Arc<ProverDb>,
    prover: Arc<Prover>,
    proof_notifier: Arc<Notify>,
    max_rollup_gas_price_gwei: Option<u64>,
    rollup_subscription: Option<mpsc::Sender<BlockHeight>>,
    postgres_db: Option<Arc<tokio_postgres::Client>>,
) -> Result<()> {
    rollup_contract.client.use_latest_for_nonce = true;
    let rollup_contract = rollup_contract;

    let mut skip_waiting = true;
    loop {
        if !skip_waiting {
            tokio::select! {
                _ = tokio::time::sleep(wait_time) => {}
                _ = proof_notifier.notified() => {}
            }
        }

        skip_waiting = false;

        let contract_height = BlockHeight(rollup_contract.block_height().await?);
        let max = BlockHeight(u64::MAX);

        let Some(rollup) = prover_state_db
            .list_rollups(contract_height.next()..max)
            .next()
        else {
            info!(?contract_height, "No proofs to roll up");
            continue;
        };

        let (height, rollup) = rollup?;
        let mut postgres_missed_proof_release_lock = Option::<ScopeGuard<(), _>>::None;

        let rollup_contract_root_hash =
            Element::from_be_bytes(rollup_contract.root_hash().await?.0);
        let rollup = if rollup.old_root() != rollup_contract_root_hash {
            info!(
                ?contract_height,
                ?rollup_contract_root_hash,
                prover_rollup_height = ?height,
                prover_rollup_old_root = ?rollup.old_root(),
                "We do not have a proof for the next block"
            );

            if let Some(postgres_db) = postgres_db.as_ref() {
                let mut rows = postgres_db
                    .query(
                        "
                        WITH pending_rollup AS MATERIALIZED (
                            SELECT height, proof FROM rollup_proofs WHERE old_root = $1 AND now() - added_at > $2::text::interval LIMIT 1
                        ) SELECT proof FROM pending_rollup WHERE pg_try_advisory_lock(height::bigint)
                        ",
                        &[&rollup_contract_root_hash.to_be_bytes().to_vec(), &"5 minutes"]
                    )
                    .await?;
                if let Some(row) = rows.pop() {
                    let proof: RollupInput =
                        borsh::BorshDeserialize::try_from_slice(&row.get::<_, Vec<u8>>(0)).unwrap();

                    let postgres_db = Arc::clone(postgres_db);
                    let release_lock_proof = proof.clone();
                    postgres_missed_proof_release_lock.replace(scopeguard::guard((), move |_| {
                        let proof = release_lock_proof;
                        tokio::spawn(async move {
                            postgres_db
                                .execute(
                                    "SELECT pg_advisory_unlock($1)",
                                    &[&(proof.height() as i64)],
                                )
                                .await
                                .unwrap();
                        });
                    }));

                    info!(
                        ?contract_height,
                        ?rollup_contract_root_hash,
                        prover_rollup_height = proof.height(),
                        prover_rollup_old_root = ?proof.old_root(),
                        "Found a proof for the next block in postgres"
                    );

                    proof
                } else {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            rollup
        };

        if let Some(max_gas_price_gwei) = max_rollup_gas_price_gwei {
            let current_gas_price = match rollup_contract.client.fast_gas_price().await {
                Ok(price) => price,
                Err(err) => {
                    let err = ContractsError::from(err);
                    warn!(?err, "Failed to fetch gas price; skipping rollup attempt");
                    continue;
                }
            };
            if gas_price_exceeds_threshold(max_gas_price_gwei, current_gas_price) {
                continue;
            }
        }

        let pending_nonce = rollup_contract
            .client
            .get_nonce(
                rollup_contract.signer_address,
                web3::types::BlockNumber::Pending,
            )
            .await
            .map_err(Error::FailedToGetNonce)?;
        let latest_nonce = rollup_contract
            .client
            .get_nonce(
                rollup_contract.signer_address,
                web3::types::BlockNumber::Latest,
            )
            .await
            .map_err(Error::FailedToGetNonce)?;
        let txn_is_pending = pending_nonce > latest_nonce;
        if txn_is_pending {
            info!(
                ?pending_nonce,
                ?latest_nonce,
                "Skipping roll up and instead waiting for pending transaction to be mined or dropped from mempool"
            );
            continue;
        }

        info!(counter.rolling_up_height = ?height, "Rolling up proof");

        if let Err(err) = prover.rollup(&rollup).await {
            if let prover::Error::RollupTransactionTimeout = err {
                // This should exit the process
                return Err(err.into());
            }

            error!(?err, ?rollup, "Failed to roll up proof");
            continue;
        }

        info!(counter.rolled_up_height = ?height, "Rolled up proof");

        if let Some(rollup_subscription) = &rollup_subscription {
            rollup_subscription.send(height).await?;
        }

        // There might be more rollups to do, so after a success,
        // don't wait a constant duration
        skip_waiting = true;
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        str::FromStr,
        sync::{Arc, Mutex},
    };

    use crate::{
        Block,
        block::{BlockContent, BlockHeader, BlockState},
    };

    use super::*;
    use contracts::{SecretKey, util::convert_element_to_h256};
    use doomslug::ApprovalContent;
    use primitives::peer::PeerIdSigner;
    use tempdir::TempDir;
    use testutil::{
        ACCOUNT_1_SK,
        eth::{EthNode, EthNodeOptions},
    };
    use zk_circuits::BbBackend;
    use zk_primitives::AggFinalProof;

    #[derive(Clone)]
    struct BufferingWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for BufferingWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn logs_and_skips_when_gas_price_exceeds_threshold() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = {
            let buffer = Arc::clone(&buffer);
            move || BufferingWriter {
                buffer: Arc::clone(&buffer),
            }
        };

        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .without_time()
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        assert!(super::gas_price_exceeds_threshold(
            1,
            U256::from(2_000_000_000u64)
        ));
        assert!(!super::gas_price_exceeds_threshold(
            3,
            U256::from(2_000_000_000u64)
        ));

        let logs = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("Skipping rollup due to high gas price"),
            "expected log to contain skip message, got: {logs}"
        );
    }

    #[ignore]
    #[tokio::test]
    async fn test_rollup() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::try_new("info").unwrap())
            .init();

        let tempdir = TempDir::new("rollup").unwrap();

        let eth_node = EthNode::new(EthNodeOptions {
            use_noop_verifier: true,
            ..Default::default()
        })
        .run_and_deploy()
        .await;

        let evm_secret_key = SecretKey::from_str(ACCOUNT_1_SK).unwrap();

        let signer = PeerIdSigner::new(secp256k1::SecretKey::from_str(ACCOUNT_1_SK).unwrap());

        let rollup_contract = RollupContract::from_eth_node(&eth_node, evm_secret_key)
            .await
            .unwrap();

        let prover_db = Arc::new(ProverDb::create_or_load(tempdir.path()).unwrap());
        let bb_backend: Arc<dyn BbBackend> = Arc::new(barretenberg_cli::CliBackend);
        let prover = Prover::new(rollup_contract.clone(), bb_backend);
        let proof_notifier = Arc::new(Notify::new());

        let (rollup_height_sender, mut rollup_height_receiver) = mpsc::channel(1);

        let rollup_worker = run_rollup_worker(
            Duration::from_secs(3),
            rollup_contract.clone(),
            Arc::clone(&prover_db),
            Arc::new(prover),
            Arc::clone(&proof_notifier),
            None,
            Some(rollup_height_sender),
            None,
        );

        let mut rollup_worker = Box::pin(rollup_worker);

        let mut last_block = Block::genesis();
        // let mut last_root = empty_tree_hash(MERKLE_TREE_DEPTH);
        for height in [1, 2, 3] {
            let new_root = Element::new(height);
            let block = BlockContent {
                header: BlockHeader {
                    height: BlockHeight(height),
                    last_block_hash: last_block.hash(),
                    ..Default::default()
                },
                state: BlockState::new(new_root, Vec::new()),
            }
            .to_block(&signer);

            let approval =
                ApprovalContent::new_endorsement(&block.hash(), *block.content.header.height + 1)
                    .to_approval_validated(&signer);

            let other_hash = block.content.header_hash();

            prover_db
                .set_rollup(
                    BlockHeight(height),
                    RollupInput::new(
                        AggFinalProof::default(),
                        height,
                        *other_hash.inner(),
                        vec![approval.signature],
                    ),
                )
                .unwrap();
            proof_notifier.notify_waiters();

            tokio::select! {
                res = &mut rollup_worker => {
                    panic!("Rollup worker should not have finished. Res: {res:?}")
                }
                rollup_height = rollup_height_receiver.recv() => {
                    assert_eq!(rollup_height.unwrap(), BlockHeight(height));
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
            }

            assert_eq!(rollup_contract.block_height().await.unwrap(), height);
            assert_eq!(
                rollup_contract.root_hash().await.unwrap(),
                convert_element_to_h256(&new_root)
            );

            last_block = block;
            // last_root = new_root;
        }
    }
}
