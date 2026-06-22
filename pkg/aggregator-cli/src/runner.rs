// lint-long-file-override allow-max-lines=300
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use aggregator::SmirkRollupTree;
use aggregator_interface::{
    Aggregator as AggregatorTrait, PreparationOutcome, PreparedBatch, PrioritizableBbBackend,
    ProvenBatch, RollupTree,
};
use async_fn_stream::try_fn_stream;
use contextful::ResultContextExt;
use element::Element;
use futures::{Stream, StreamExt};
use node_client_http::NodeClientHttp;
use primitives::block_height::BlockHeight;
use tokio::{time::Instant, time::sleep};
use tracing::{error, info, warn};

use crate::error::{Error, Result};

fn prepare_stream(
    aggregator: Arc<dyn AggregatorTrait>,
) -> impl Stream<Item = Result<PreparationOutcome>> {
    try_fn_stream(|emitter| async move {
        loop {
            let outcome = aggregator
                .prepare_next_batch()
                .await
                .context("prepare next batch")?;

            if let PreparationOutcome::Success(batch) = &outcome {
                let start_height = batch.prepared_blocks.first().unwrap().height;
                let end_height = batch.prepared_blocks.last().unwrap().height;
                info!(
                    start_height,
                    end_height,
                    count = batch.prepared_blocks.len(),
                    "prepared next batch"
                );
            }

            emitter.emit(outcome).await;
        }
    })
}

fn proof_stream(
    aggregator: Arc<dyn AggregatorTrait>,
    bb_backend: Arc<dyn PrioritizableBbBackend>,
    pipeline_depth: usize,
    prepare_stream: impl Stream<Item = (usize, Result<PreparedBatch>)>,
) -> impl Stream<Item = Result<(usize, ProvenBatch)>> {
    prepare_stream
        .map(move |(seq, batch_res)| {
            let aggregator = Arc::clone(&aggregator);
            let bb_backend = Arc::clone(&bb_backend);
            async move {
                let batch = batch_res?;
                let start_height = batch.prepared_blocks.first().unwrap().height;
                let end_height = batch.prepared_blocks.last().unwrap().height;

                let prioritized_backend = bb_backend.with_priority(start_height);

                info!(start_height, end_height, "proving batch");
                let start = Instant::now();
                let res = aggregator
                    .prove_batch(batch, prioritized_backend)
                    .await
                    .context("prove batch")
                    .map_err(Error::from);

                if res.is_ok() {
                    info!(
                        start_height,
                        end_height,
                        duration_ms = start.elapsed().as_millis(),
                        "finished proof for batch"
                    );
                } else {
                    error!(start_height, end_height, "failed to prove batch");
                }
                res.map(|proven| (seq, proven))
            }
        })
        .buffer_unordered(pipeline_depth)
}

fn reorder_stream(
    proof_stream: impl Stream<Item = Result<(usize, ProvenBatch)>>,
) -> impl Stream<Item = Result<ProvenBatch>> {
    try_fn_stream(|emitter| async move {
        let mut buffer = BTreeMap::new();
        let mut next_seq = 0;
        tokio::pin!(proof_stream);

        while let Some(res) = proof_stream.next().await {
            let (seq, proven) = res?;
            buffer.insert(seq, proven);

            while let Some(proven) = buffer.remove(&next_seq) {
                emitter.emit(proven).await;
                next_seq += 1;
            }
        }
        Ok(())
    })
}

fn submission_stream(
    aggregator: Arc<dyn AggregatorTrait>,
    proof_stream: impl Stream<Item = Result<ProvenBatch>>,
) -> impl Stream<Item = Result<()>> {
    proof_stream.then(move |proven_res| {
        let aggregator = Arc::clone(&aggregator);
        async move {
            let proven = proven_res?;
            let start_height = proven.blocks.first().unwrap().block.content.header.height.0;
            let end_height = proven.blocks.last().unwrap().block.content.header.height.0;

            info!(start_height, end_height, "submitting batch to contract");
            aggregator
                .submit_batch(proven)
                .await
                .context("failed to submit batch")?;
            info!(start_height, end_height, "successfully submitted batch");
            Ok(())
        }
    })
}

fn build_pipeline(
    aggregator: Arc<dyn AggregatorTrait>,
    bb_backend: Arc<dyn PrioritizableBbBackend>,
    pipeline_depth: usize,
    prepare_stream: impl Stream<Item = (usize, Result<PreparedBatch>)>,
) -> impl Stream<Item = Result<()>> {
    let proof = proof_stream(
        Arc::clone(&aggregator),
        bb_backend,
        pipeline_depth,
        prepare_stream,
    );
    let ordered_proof = reorder_stream(proof);
    submission_stream(aggregator, ordered_proof)
}

pub async fn run_once(
    aggregator: Arc<dyn AggregatorTrait>,
    bb_backend: Arc<dyn PrioritizableBbBackend>,
    pipeline_depth: usize,
) -> Result<()> {
    let prepare = prepare_stream(Arc::clone(&aggregator))
        .scan((), |_, res| async move {
            match res {
                Ok(PreparationOutcome::Success(batch)) => Some(Ok(batch)),
                Ok(PreparationOutcome::InsufficientBlocks {
                    start_height,
                    available,
                    required,
                }) => {
                    info!(
                        start_height,
                        available, required, "insufficient blocks, stopping"
                    );
                    None
                }
                Err(err) => Some(Err(err)),
            }
        })
        .enumerate();
    let submission = build_pipeline(aggregator, bb_backend, pipeline_depth, prepare);
    tokio::pin!(submission);

    while let Some(res) = submission.next().await {
        res?;
    }

    Ok(())
}

pub async fn run_loop(
    aggregator: Arc<dyn AggregatorTrait>,
    bb_backend: Arc<dyn PrioritizableBbBackend>,
    interval: Duration,
    pipeline_depth: usize,
    mut shutdown: Pin<Box<dyn Future<Output = ()> + Send>>,
) -> Result<()> {
    info!(pipeline_depth, "starting aggregator loop");

    let prepare = prepare_stream(Arc::clone(&aggregator))
        .filter_map(|res| async move {
            match res {
                Ok(PreparationOutcome::Success(batch)) => Some(Ok(batch)),
                Ok(PreparationOutcome::InsufficientBlocks {
                    start_height,
                    available,
                    required,
                }) => {
                    info!(
                        start_height,
                        available, required, "insufficient blocks, waiting..."
                    );
                    sleep(interval).await;
                    None
                }
                Err(err) => {
                    warn!(?err, "failed to prepare next batch, retrying");
                    sleep(interval).await;
                    None
                }
            }
        })
        .enumerate();
    let submission = build_pipeline(aggregator, bb_backend, pipeline_depth, prepare);
    tokio::pin!(submission);

    loop {
        tokio::select! {
            biased;
            res = submission.next() => {
                match res {
                    Some(Ok(())) => {},
                    Some(Err(err)) => return Err(err),
                    None => break,
                }
            }
            _ = &mut shutdown => {
                info!("shutting down aggregator");
                return Ok(());
            }
        }
    }

    Ok(())
}

pub async fn build_rollup_tree(
    client: &NodeClientHttp,
    rolled_height: u64,
    expected_root: Element,
) -> Result<SmirkRollupTree> {
    info!(
        rolled_height,
        ?expected_root,
        "building rollup tree from snapshot"
    );
    let snapshot = client
        .block_tree(BlockHeight(rolled_height))
        .await
        .context("fetch block tree snapshot")?;

    if snapshot.root_hash != expected_root {
        return Err(Error::RootMismatch {
            node_root: snapshot.root_hash,
            contract_root: expected_root,
        });
    }

    let mut tree = SmirkRollupTree::new();
    let inserts: Vec<_> = snapshot
        .elements
        .into_iter()
        .map(|element| (element, rolled_height))
        .collect();

    let count = inserts.len();
    tree.insert(&inserts)
        .context("seed rollup tree insert batch")?;
    tree.set_height(rolled_height);

    info!(count, "successfully seeded rollup tree");
    Ok(tree)
}
