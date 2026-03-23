use super::*;
use aggregator_interface::{
    BlockProver as BlockProverTrait, RollupContract as RollupContractTrait, RollupTree,
};
use element::Element;
use node_interface::{ListBlocksResponse, NodeClient, NodeClientMock};
use primitives::pagination::OpaqueCursor;
use std::sync::Arc;
use unimock::{MockFn, Unimock, matching};

fn new_aggregator(node_client: Arc<dyn NodeClient>) -> Aggregator {
    let rollup_contract: Arc<dyn RollupContractTrait> = Arc::new(Unimock::new(()));
    let block_prover: Arc<dyn BlockProverTrait> = Arc::new(Unimock::new(()));
    let rollup_tree: Box<dyn RollupTree> = Box::new(Unimock::new(()));
    let agg_agg_circuit: Arc<dyn AggAggCircuitInterface> = Arc::new(Unimock::new(()));
    let agg_final_circuit: Arc<dyn AggFinalCircuitInterface> = Arc::new(Unimock::new(()));

    Aggregator::new(
        node_client,
        rollup_contract,
        block_prover,
        rollup_tree,
        DEFAULT_BLOCK_BATCH_SIZE,
        TEST_GAS_PER_BURN_CALL,
        agg_agg_circuit,
        agg_final_circuit,
    )
}

#[tokio::test]
async fn fetch_blocks_exact_refetches_until_limit_is_met() {
    let first_block = block_with_single_txn(2, Element::new(10), vec![signature(1)], 101);
    let second_block = block_with_single_txn(4, Element::new(20), vec![signature(2)], 202);

    let node_client: Arc<dyn NodeClient> = Arc::new(Unimock::new((
        NodeClientMock::blocks
            .next_call(matching!((BlockHeight(2), 2, true)))
            .returns(Ok(ListBlocksResponse {
                blocks: vec![first_block],
                cursor: OpaqueCursor::default(),
            })),
        NodeClientMock::blocks
            .next_call(matching!((BlockHeight(3), 1, true)))
            .returns(Ok(ListBlocksResponse {
                blocks: vec![second_block],
                cursor: OpaqueCursor::default(),
            })),
    )));
    let aggregator = new_aggregator(node_client);

    let blocks = aggregator.fetch_blocks_exact(2, 2).await.unwrap();
    let heights = blocks
        .into_iter()
        .map(|block| block.block.content.header.height.0)
        .collect::<Vec<_>>();

    assert_eq!(heights, vec![2, 4]);
}

#[tokio::test]
async fn fetch_blocks_exact_retries_when_refetch_is_empty() {
    let first_block = block_with_single_txn(2, Element::new(10), vec![signature(1)], 101);
    let second_block = block_with_single_txn(5, Element::new(20), vec![signature(2)], 202);

    let node_client: Arc<dyn NodeClient> = Arc::new(Unimock::new((
        NodeClientMock::blocks
            .next_call(matching!((BlockHeight(2), 2, true)))
            .returns(Ok(ListBlocksResponse {
                blocks: vec![first_block],
                cursor: OpaqueCursor::default(),
            })),
        NodeClientMock::blocks
            .next_call(matching!((BlockHeight(3), 1, true)))
            .returns(Ok(ListBlocksResponse {
                blocks: vec![],
                cursor: OpaqueCursor::default(),
            })),
        NodeClientMock::blocks
            .next_call(matching!((BlockHeight(3), 1, true)))
            .returns(Ok(ListBlocksResponse {
                blocks: vec![second_block],
                cursor: OpaqueCursor::default(),
            })),
    )));
    let aggregator = new_aggregator(node_client);

    let blocks = aggregator.fetch_blocks_exact(2, 2).await.unwrap();
    let heights = blocks
        .into_iter()
        .map(|block| block.block.content.header.height.0)
        .collect::<Vec<_>>();

    assert_eq!(heights, vec![2, 5]);
}
