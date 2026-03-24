// lint-long-file-override allow-max-lines=300
use super::*;
use aggregator_interface::{
    Aggregator as AggregatorTrait, BlockProver as BlockProverTrait, BlockProverMock,
    PreparationOutcome, PreparedBlock, RollupContract as RollupContractTrait, RollupContractMock,
    RollupTree, RollupTreeMock,
};
use barretenberg_interface::BbBackend;
use element::Element;
use node_interface::{ListBlocksResponse, NodeClient, NodeClientMock};
use primitives::pagination::OpaqueCursor;
use std::sync::Arc;
use unimock::{MockFn, Unimock, matching};
use zk_circuits::Error as CircuitError;
use zk_circuits::Proof;
use zk_circuits::circuits::generated::agg_agg::AggAggInput as CircuitAggAggInput;
use zk_circuits::circuits::generated::agg_final::AggFinalInput as CircuitAggFinalInput;
use zk_primitives::{
    AggAggProof, AggAggPublicInput, AggFinalProof, AggFinalPublicInput, OracleProofBytes,
    ProofBytes,
};

use crate::CircuitMock;

#[tokio::test]
async fn step_rolls_two_blocks_and_submits_rollup() {
    let rolled_height = 10;
    let rolled_root = Element::new(100);
    let first_height = rolled_height + 1;
    let second_height = rolled_height + 3;
    let first_root = Element::new(200);
    let second_root = Element::new(300);
    let next_block_signatures = vec![signature(1), signature(2)];

    let first_block = block_with_single_txn(first_height, first_root, vec![signature(3)], 11);
    let second_block = block_with_single_txn(second_height, second_root, vec![signature(4)], 22);
    let third_height = second_height + 1;
    let third_root = Element::new(400);
    let third_block =
        block_with_single_txn(third_height, third_root, next_block_signatures.clone(), 33);

    let node_blocks_response = ListBlocksResponse {
        blocks: vec![first_block.clone(), second_block.clone()],
        cursor: OpaqueCursor::default(),
    };
    let next_block_response = ListBlocksResponse {
        blocks: vec![third_block.clone()],
        cursor: OpaqueCursor::default(),
    };

    let first_block_proof = block_agg_proof(rolled_root, first_root, Element::new(501));
    let second_block_proof = block_agg_proof(first_root, second_root, Element::new(502));

    let mut aggregated_messages = [Element::ZERO; 1000];
    aggregated_messages[0] = Element::new(5);
    aggregated_messages[1] = Element::new(6);

    let aggregated_agg_public_inputs = AggAggPublicInput {
        verification_key_hash: [Element::new(7), Element::new(8)],
        old_root: rolled_root,
        new_root: second_root,
        commit_hash: Element::new(999),
        messages: aggregated_messages,
    };

    let aggregated_final_public_inputs = AggFinalPublicInput {
        old_root: rolled_root,
        new_root: second_root,
        commit_hash: aggregated_agg_public_inputs.commit_hash,
        messages: aggregated_messages.to_vec(),
    };
    let aggregated_agg_inputs_ref = leak(aggregated_agg_public_inputs.clone());
    let aggregated_final_inputs_ref = leak(aggregated_final_public_inputs.clone());

    let aggregated_agg_proof = AggAggProof {
        proof: ProofBytes::default(),
        public_inputs: aggregated_agg_public_inputs.clone(),
        kzg: vec![],
    };
    let expected_proof_bytes = vec![1u8; AGG_FINAL_PROOF_ELEMENT_COUNT * BYTES_PER_ELEMENT];
    let final_proof = AggFinalProof {
        proof: OracleProofBytes(expected_proof_bytes.clone()),
        public_inputs: aggregated_final_public_inputs.clone(),
        kzg: vec![],
    };

    let first_height_ref = leak(first_height);
    let second_height_ref = leak(second_height);
    let rolled_root_ref = leak(rolled_root);
    let second_root_ref = leak(second_root);
    let commit_hash_ref = leak(aggregated_agg_public_inputs.commit_hash);
    let expected_messages = aggregated_final_public_inputs.messages.clone();
    let expected_messages_ref = leak(expected_messages.clone());
    let expected_proof_bytes_ref = leak(expected_proof_bytes.clone());
    let expected_other_hash = utils::block_header_hash(&second_block.block.content.header).unwrap();
    let expected_other_hash_ref = leak(expected_other_hash);
    let expected_signature_bytes: Vec<Vec<u8>> = next_block_signatures
        .iter()
        .map(|sig| sig.inner().to_vec())
        .collect();
    let expected_signature_bytes_ref = leak(expected_signature_bytes.clone());
    let node_blocks_response_ref = leak(node_blocks_response.clone());
    let next_block_response_ref = leak(next_block_response.clone());
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
            .returns(Ok(next_block_response_ref.clone())),
    )));

    let rollup_contract: Arc<dyn RollupContractTrait> = Arc::new(Unimock::new(
        RollupContractMock::submit_rollup
            .next_call(matching!((rollup)
                if rollup.height == *second_height_ref
                    && rollup.old_root == *rolled_root_ref
                    && rollup.new_root == *second_root_ref
                    && rollup.commit_hash == *commit_hash_ref
                    && rollup.utxo_messages == *expected_messages_ref
                    && rollup.proof == *expected_proof_bytes_ref
                    && rollup.other_hash == *expected_other_hash_ref
                    && rollup.signatures == *expected_signature_bytes_ref
                    && rollup.gas_per_burn_call == GAS_PER_BURN_CALL))
            .returns(Ok(())),
    ));

    let first_prepared = PreparedBlock {
        height: *first_height_ref,
        chunks: [
            dummy_chunk(*rolled_root_ref, first_root),
            dummy_chunk(Element::ZERO, Element::ZERO),
        ],
    };
    let second_prepared = PreparedBlock {
        height: *second_height_ref,
        chunks: [
            dummy_chunk(first_root, *second_root_ref),
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
                if agg_input.old_root == aggregated_agg_inputs_ref.old_root
                    && agg_input.new_root == aggregated_agg_inputs_ref.new_root))
            .returns(Ok::<_, CircuitError>(Proof::from(
                aggregated_agg_proof.clone(),
            ))),
    ));
    let agg_final_circuit: Arc<dyn AggFinalCircuitInterface> = Arc::new(Unimock::new(
        CircuitMock::prove
            .with_types::<CircuitAggFinalInput, _>()
            .next_call(matching!((final_input, _)
                if final_input.old_root == aggregated_final_inputs_ref.old_root
                    && final_input.new_root == aggregated_final_inputs_ref.new_root))
            .returns(Ok::<_, CircuitError>(Proof::from(final_proof.clone()))),
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

    let preparation = aggregator.prepare_next_batch().await.unwrap();
    let batch = match preparation {
        PreparationOutcome::Success(batch) => batch,
        _ => panic!("Expected success"),
    };
    let proven = aggregator
        .prove_batch(batch, Arc::clone(&bb_backend))
        .await
        .unwrap();
    aggregator.submit_batch(proven).await.unwrap();
}
