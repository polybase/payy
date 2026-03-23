// lint-long-file-override allow-max-lines=300
use super::*;
use aggregator_interface::{
    Aggregator as AggregatorTrait, BlockProver as BlockProverTrait, BlockProverMock,
    PreparationOutcome, PreparedBlock, RollupTree, RollupTreeMock,
};
use barretenberg_interface::BbBackend;
use element::Element;
use node_interface::{ListBlocksResponse, NodeClient, NodeClientMock};
use primitives::pagination::OpaqueCursor;
use std::sync::Arc;
use unimock::{MockFn, Unimock, matching};
use zk_circuits::circuits::generated::agg_agg::AggAggInput as CircuitAggAggInput;
use zk_circuits::circuits::generated::agg_final::AggFinalInput as CircuitAggFinalInput;
use zk_circuits::{Error as CircuitError, Proof};
use zk_primitives::{
    AggAggProof, AggAggPublicInput, AggFinalProof, AggFinalPublicInput, OracleProofBytes,
    ProofBytes,
};

use crate::CircuitMock;

#[tokio::test]
async fn step_errors_when_next_block_missing() {
    let rolled_height = 10;
    let rolled_root = Element::new(100);
    let first_height = rolled_height + 1;
    let second_height = rolled_height + 3;
    let first_root = Element::new(200);
    let second_root = Element::new(300);

    let first_block = block_with_single_txn(first_height, first_root, vec![signature(1)], 11);
    let second_block = block_with_single_txn(second_height, second_root, vec![signature(2)], 22);

    let node_blocks_response = ListBlocksResponse {
        blocks: vec![first_block.clone(), second_block.clone()],
        cursor: OpaqueCursor::default(),
    };
    let empty_next_block_response = ListBlocksResponse {
        blocks: vec![],
        cursor: OpaqueCursor::default(),
    };

    let first_block_proof = block_agg_proof(rolled_root, first_root, Element::new(601));
    let second_block_proof = block_agg_proof(first_root, second_root, Element::new(602));

    let mut aggregated_messages = [Element::ZERO; 1000];
    aggregated_messages[0] = Element::new(9);

    let aggregated_agg_public_inputs = AggAggPublicInput {
        verification_key_hash: [Element::new(10), Element::new(11)],
        old_root: rolled_root,
        new_root: second_root,
        commit_hash: Element::new(700),
        messages: aggregated_messages,
    };

    let aggregated_final_public_inputs = AggFinalPublicInput {
        old_root: rolled_root,
        new_root: second_root,
        commit_hash: aggregated_agg_public_inputs.commit_hash,
        messages: aggregated_messages.to_vec(),
    };

    let aggregated_agg_proof = AggAggProof {
        proof: ProofBytes::default(),
        public_inputs: aggregated_agg_public_inputs.clone(),
        kzg: vec![],
    };
    let aggregated_agg_inputs_err_ref = leak(aggregated_agg_public_inputs.clone());
    let aggregated_final_inputs_err_ref = leak(aggregated_final_public_inputs.clone());

    let agg_final_proof = AggFinalProof {
        proof: OracleProofBytes(vec![2u8; AGG_FINAL_PROOF_ELEMENT_COUNT * BYTES_PER_ELEMENT]),
        public_inputs: aggregated_final_public_inputs.clone(),
        kzg: vec![],
    };

    let first_height_ref = leak(first_height);
    let second_height_ref = leak(second_height);
    let node_blocks_response_ref = leak(node_blocks_response.clone());
    let empty_next_block_response_ref = leak(empty_next_block_response.clone());
    let first_block_proof_ref = leak(first_block_proof.clone());
    let second_block_proof_ref = leak(second_block_proof.clone());
    let bb_backend: Arc<dyn BbBackend> = Arc::new(Unimock::new(()));

    let node_client: Arc<dyn NodeClient> = Arc::new(Unimock::new((
        NodeClientMock::blocks
            .next_call(matching!((start_height, limit, skip_empty)
                if *start_height == BlockHeight(*first_height_ref)
                    && *limit >= DEFAULT_BLOCK_BATCH_SIZE
                    && *skip_empty))
            .returns(Ok(node_blocks_response_ref.clone())),
        NodeClientMock::blocks
            .next_call(matching!((start_height, limit, skip_empty)
                if *start_height == BlockHeight(*second_height_ref + 1)
                    && *limit == 1
                    && !*skip_empty))
            .returns(Ok(empty_next_block_response_ref.clone())),
    )));

    let rollup_contract: Arc<dyn aggregator_interface::RollupContract> = Arc::new(Unimock::new(()));

    let first_prepared = PreparedBlock {
        height: *first_height_ref,
        chunks: [
            dummy_chunk(Element::ZERO, Element::ZERO),
            dummy_chunk(Element::ZERO, Element::ZERO),
        ],
    };
    let second_prepared = PreparedBlock {
        height: *second_height_ref,
        chunks: [
            dummy_chunk(Element::ZERO, Element::ZERO),
            dummy_chunk(Element::ZERO, Element::ZERO),
        ],
    };
    let block_prover: Arc<dyn BlockProverTrait> = Arc::new(Unimock::new((
        BlockProverMock::prepare
            .next_call(matching!((height, _tree) if *height == *first_height_ref))
            .returns(Ok(first_prepared.clone())),
        BlockProverMock::prepare
            .next_call(matching!((height, _tree) if *height == *second_height_ref))
            .returns(Ok(second_prepared.clone())),
        BlockProverMock::prove
            .next_call(matching!((prepared, _) if prepared.height == *first_height_ref))
            .returns(Ok(first_block_proof_ref.clone())),
        BlockProverMock::prove
            .next_call(matching!((prepared, _) if prepared.height == *second_height_ref))
            .returns(Ok(second_block_proof_ref.clone())),
    )));

    let rollup_tree: Box<dyn RollupTree> = Box::new(Unimock::new((
        RollupTreeMock::height
            .next_call(matching!(()))
            .returns(rolled_height),
        RollupTreeMock::root_hash
            .next_call(matching!(()))
            .returns(rolled_root),
        RollupTreeMock::height
            .next_call(matching!(()))
            .returns(rolled_height),
        RollupTreeMock::root_hash
            .next_call(matching!(()))
            .returns(rolled_root),
        RollupTreeMock::root_hash
            .next_call(matching!(()))
            .returns(second_root),
        RollupTreeMock::set_height
            .next_call(matching!((height) if *height == *second_height_ref))
            .returns(()),
    )));
    let agg_agg_circuit: Arc<dyn AggAggCircuitInterface> = Arc::new(Unimock::new(
        CircuitMock::prove
            .with_types::<CircuitAggAggInput, _>()
            .next_call(matching!((agg_input, _)
                if agg_input.old_root == aggregated_agg_inputs_err_ref.old_root
                    && agg_input.new_root == aggregated_agg_inputs_err_ref.new_root))
            .returns(Ok::<_, CircuitError>(Proof::from(
                aggregated_agg_proof.clone(),
            ))),
    ));
    let agg_final_circuit: Arc<dyn AggFinalCircuitInterface> = Arc::new(Unimock::new(
        CircuitMock::prove
            .with_types::<CircuitAggFinalInput, _>()
            .next_call(matching!((final_input, _)
                if final_input.old_root == aggregated_final_inputs_err_ref.old_root
                    && final_input.new_root == aggregated_final_inputs_err_ref.new_root))
            .returns(Ok::<_, CircuitError>(Proof::from(agg_final_proof.clone()))),
    ));
    let aggregator = Aggregator::new(
        node_client,
        rollup_contract,
        block_prover,
        rollup_tree,
        DEFAULT_BLOCK_BATCH_SIZE,
        TEST_GAS_PER_BURN_CALL,
        agg_agg_circuit,
        agg_final_circuit,
    );

    let prepared = aggregator.prepare_next_batch().await.unwrap();
    let batch = match prepared {
        PreparationOutcome::Success(batch) => batch,
        _ => panic!("Expected success"),
    };
    let proven = aggregator
        .prove_batch(batch, Arc::clone(&bb_backend))
        .await
        .unwrap();
    let err = aggregator.submit_batch(proven).await.unwrap_err();
    assert!(matches!(
        err,
        aggregator_interface::Error::MissingApprovalBlock
    ));
}
