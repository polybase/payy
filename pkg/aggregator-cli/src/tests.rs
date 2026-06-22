// lint-long-file-override allow-max-lines=400
use std::{sync::Arc, time::Duration};

use aggregator_interface::{
    AggregatorMock, PreparationOutcome, PreparedBatch, PreparedBlock, PreparedChunk,
    PrioritizableBbBackend, PrioritizableBbBackendMock, ProvenBatch,
};
use async_trait::async_trait;
use clap::Parser;
use element::Element;
use futures::FutureExt;
use futures::future::BoxFuture;
use node_interface::{Block, BlockContent, BlockHeader, BlockState, BlockWithInfo};
use primitives::{block_height::BlockHeight, hash::CryptoHash, sig::Signature};
use unimock::{MockFn, Unimock, matching, unimock};
use zk_primitives::{AggFinalProof, UtxoProofBundleWithMerkleProofs};

use crate::config::{
    Config, select_rollup_tree_seed_height, validate_rollup_root_height_overrides,
};
use crate::runner::{run_loop, run_once};

#[unimock(api = ProofMock)]
trait AsyncProveBatch {
    fn prove_batch(
        &self,
        batch: PreparedBatch,
    ) -> BoxFuture<'static, Result<ProvenBatch, aggregator_interface::Error>>;
}

/// Helper struct to allow `unimock` to return a `BoxFuture` for `prove_batch`.
/// This is necessary because `Aggregator` is an `#[async_trait]`, and `unimock`'s default behavior
/// for `answers` expects a return value of `Result`, not allowing us to easily simulate delays
/// by returning a pending Future manually.
struct AggregatorWithAsyncProveBatch(Unimock);

#[async_trait]
impl aggregator_interface::Aggregator for AggregatorWithAsyncProveBatch {
    async fn prepare_next_batch(&self) -> Result<PreparationOutcome, aggregator_interface::Error> {
        aggregator_interface::Aggregator::prepare_next_batch(&self.0).await
    }

    async fn prove_batch(
        &self,
        batch: aggregator_interface::PreparedBatch,
        _bb_backend: Arc<dyn barretenberg_interface::BbBackend>,
    ) -> Result<ProvenBatch, aggregator_interface::Error> {
        AsyncProveBatch::prove_batch(&self.0, batch).await
    }

    async fn submit_batch(&self, batch: ProvenBatch) -> Result<(), aggregator_interface::Error> {
        aggregator_interface::Aggregator::submit_batch(&self.0, batch).await
    }
}

#[tokio::test]
async fn test_out_of_order_proof_completion_reordering() {
    let batch1 = create_batch(1, 10, 20);
    let batch2 = create_batch(2, 20, 30);
    let batch3 = create_batch(3, 30, 40);
    let batch4 = create_batch(4, 40, 50);

    let proven1 = create_proven(1, [1; 32]);
    let proven2 = create_proven(2, [2; 32]);
    let proven3 = create_proven(3, [3; 32]);
    let proven4 = create_proven(4, [4; 32]);

    let batch1_root = leak(batch1.new_root);
    let batch2_root = leak(batch2.new_root);
    let batch3_root = leak(batch3.new_root);
    let batch4_root = leak(batch4.new_root);
    let proven1_hash = leak(proven1.other_hash);
    let proven2_hash = leak(proven2.other_hash);
    let proven3_hash = leak(proven3.other_hash);
    let proven4_hash = leak(proven4.other_hash);

    let unimock = Unimock::new((
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::Success(batch1))),
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::Success(batch2))),
        ProofMock::prove_batch
            .next_call(matching!((batch) if batch.new_root == *batch1_root))
            .answers(leak(move |_, _| {
                let proven1 = proven1.clone();
                async move {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok(proven1)
                }
                .boxed()
            })),
        ProofMock::prove_batch
            .next_call(matching!((batch) if batch.new_root == *batch2_root))
            .answers(leak(move |_, _| {
                futures::future::ready(Ok(proven2.clone())).boxed()
            })),
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::Success(batch3))),
        ProofMock::prove_batch
            .next_call(matching!((batch) if batch.new_root == *batch3_root))
            .answers(leak(move |_, _| {
                futures::future::ready(Ok(proven3.clone())).boxed()
            })),
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::Success(batch4))),
        ProofMock::prove_batch
            .next_call(matching!((batch) if batch.new_root == *batch4_root))
            .answers(leak(move |_, _| {
                futures::future::ready(Ok(proven4.clone())).boxed()
            })),
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::InsufficientBlocks {
                start_height: 5,
                available: 0,
                required: 2,
            })),
        AggregatorMock::submit_batch
            .next_call(matching!((proven) if proven.other_hash == *proven1_hash))
            .returns(Ok(())),
        AggregatorMock::submit_batch
            .next_call(matching!((proven) if proven.other_hash == *proven2_hash))
            .returns(Ok(())),
        AggregatorMock::submit_batch
            .next_call(matching!((proven) if proven.other_hash == *proven3_hash))
            .returns(Ok(())),
        AggregatorMock::submit_batch
            .next_call(matching!((proven) if proven.other_hash == *proven4_hash))
            .returns(Ok(())),
    ));

    let aggregator = Arc::new(AggregatorWithAsyncProveBatch(unimock));
    let bb_backend = mock_prioritizable_backend();

    run_once(aggregator, bb_backend, 2).await.unwrap();
}

#[tokio::test]
async fn test_run_loop_sequential() {
    let batch1 = create_batch(1, 10, 20);
    let batch2 = create_batch(2, 20, 30);

    let proven1 = create_proven(1, [1; 32]);
    let proven2 = create_proven(2, [2; 32]);

    let batch1_root = leak(batch1.new_root);
    let batch2_root = leak(batch2.new_root);
    let proven1_hash = leak(proven1.other_hash);
    let proven2_hash = leak(proven2.other_hash);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let shutdown_tx = Arc::new(std::sync::Mutex::new(Some(shutdown_tx)));

    let aggregator = Arc::new(Unimock::new((
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::Success(batch1.clone()))),
        AggregatorMock::prove_batch
            .next_call(matching!((batch, _) if batch.new_root == *batch1_root))
            .returns(Ok(proven1.clone())),
        AggregatorMock::submit_batch
            .next_call(matching!((proven) if proven.other_hash == *proven1_hash))
            .returns(Ok(())),
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::Success(batch2.clone()))),
        AggregatorMock::prove_batch
            .next_call(matching!((batch, _) if batch.new_root == *batch2_root))
            .returns(Ok(proven2.clone())),
        AggregatorMock::submit_batch
            .next_call(matching!((proven) if proven.other_hash == *proven2_hash))
            .answers(leak(move |_, _| {
                if let Some(tx) = shutdown_tx.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                Ok(())
            })),
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::InsufficientBlocks {
                start_height: 1,
                available: 0,
                required: 2,
            })),
    )));

    let bb_backend = mock_prioritizable_backend();

    let shutdown_future = Box::pin(async move {
        let _ = shutdown_rx.await;
    });

    run_loop(
        aggregator,
        bb_backend,
        Duration::from_millis(1),
        1,
        shutdown_future,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_run_loop_submission_failure() {
    let batch1 = create_batch(1, 10, 20);
    let proven1 = create_proven(1, [1; 32]);
    let batch1_root = leak(batch1.new_root);

    let aggregator = Arc::new(Unimock::new((
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::Success(batch1.clone()))),
        AggregatorMock::prove_batch
            .next_call(matching!((batch, _) if batch.new_root == *batch1_root))
            .returns(Ok(proven1.clone())),
        AggregatorMock::submit_batch
            .next_call(matching!(_))
            .returns(Err(aggregator_interface::Error::MissingApprovalBlock)),
    )));

    let bb_backend = mock_prioritizable_backend();

    let shutdown_future = Box::pin(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
    });

    let res = run_loop(
        aggregator,
        bb_backend,
        Duration::from_millis(1),
        1,
        shutdown_future,
    )
    .await;

    assert!(res.is_err());
    assert!(format!("{:?}", res.unwrap_err()).contains("failed to submit batch"));
}

#[tokio::test]
async fn test_run_once_insufficient_blocks() {
    let aggregator = Arc::new(Unimock::new(
        AggregatorMock::prepare_next_batch
            .next_call(matching!(()))
            .returns(Ok(PreparationOutcome::InsufficientBlocks {
                start_height: 1,
                available: 1,
                required: 2,
            })),
    ));

    let bb_backend = Arc::new(Unimock::new(()));

    run_once(aggregator, bb_backend, 2).await.unwrap();
}

#[test]
fn test_rollup_root_height_overrides_parse_and_select_height() {
    let config = Config::try_parse_from([
        "aggregator-cli",
        "--rollup-root-height-overrides",
        "0x2a=123,0x2b=456",
    ])
    .unwrap();

    assert_eq!(config.rollup_root_height_overrides.len(), 2);
    assert_eq!(
        select_rollup_tree_seed_height(0, Element::new(42), &config.rollup_root_height_overrides,),
        123
    );
    assert_eq!(
        select_rollup_tree_seed_height(
            99,
            Element::new(1000),
            &config.rollup_root_height_overrides,
        ),
        99
    );
}

#[test]
fn test_rollup_root_height_overrides_invalid_value() {
    let err = Config::try_parse_from([
        "aggregator-cli",
        "--rollup-root-height-overrides",
        "not-a-valid-override",
    ])
    .unwrap_err()
    .to_string();

    assert!(err.contains("ROOT_HASH=HEIGHT"));
}

#[test]
fn test_rollup_root_height_overrides_conflicting_heights_rejected() {
    let config = Config::try_parse_from([
        "aggregator-cli",
        "--rollup-root-height-overrides",
        "0x1=7,0x1=9",
    ])
    .unwrap();

    let err =
        validate_rollup_root_height_overrides(&config.rollup_root_height_overrides).unwrap_err();
    assert!(err.contains("conflicting heights for root hash"));
}

fn create_batch(height: u64, old_root: u64, new_root: u64) -> PreparedBatch {
    PreparedBatch {
        blocks: vec![],
        prepared_blocks: vec![PreparedBlock {
            height,
            chunks: [dummy_chunk(), dummy_chunk()],
        }],
        old_root: Element::new(old_root),
        new_root: Element::new(new_root),
    }
}

fn create_proven(height: u64, hash: [u8; 32]) -> ProvenBatch {
    let block = BlockWithInfo {
        block: Block {
            content: BlockContent {
                header: BlockHeader {
                    height: BlockHeight(height),
                    last_block_hash: CryptoHash::default(),
                    epoch_id: 0,
                    last_final_block_hash: CryptoHash::default(),
                    approvals: vec![],
                },
                state: BlockState {
                    root_hash: Element::ZERO,
                    txns: vec![],
                },
            },
            signature: Signature::default(),
        },
        hash: CryptoHash::default(),
        time: 0,
    };
    ProvenBatch {
        final_proof: AggFinalProof::default(),
        blocks: vec![block],
        other_hash: hash,
    }
}

fn dummy_chunk() -> PreparedChunk {
    PreparedChunk {
        old_root: Element::ZERO,
        new_root: Element::ZERO,
        bundles: std::array::from_fn(|_| UtxoProofBundleWithMerkleProofs::default()),
    }
}

fn mock_prioritizable_backend() -> Arc<dyn PrioritizableBbBackend> {
    Arc::new(Unimock::new(
        PrioritizableBbBackendMock::with_priority
            .some_call(matching!(_))
            .answers(leak(|unimock: &Unimock, _| Arc::new(unimock.clone()))),
    ))
}

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}
