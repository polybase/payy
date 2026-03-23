use super::*;
use aggregator_interface::{
    BlockProver as BlockProverTrait, RollupContract as RollupContractTrait, RollupTree,
};
use barretenberg_interface::BbBackend;
use element::Element;
use hash::hash_merge;
use node_interface::NodeClient;
use std::sync::Arc;
use unimock::{MockFn, Unimock, matching};
use zk_circuits::Error as CircuitError;
use zk_circuits::Proof;
use zk_circuits::circuits::generated::agg_agg::AggAggInput as CircuitAggAggInput;
use zk_primitives::AggAggPublicInput;

use crate::CircuitMock;

#[tokio::test]
async fn aggregates_multiple_block_proofs_hierarchically() {
    const BLOCK_BATCH_SIZE: usize = 4;
    let rolled_root = Element::new(100);
    let roots = [
        Element::new(200),
        Element::new(300),
        Element::new(400),
        Element::new(500),
    ];

    let block_proofs = vec![
        block_agg_proof(rolled_root, roots[0], Element::new(601)),
        block_agg_proof(roots[0], roots[1], Element::new(602)),
        block_agg_proof(roots[1], roots[2], Element::new(603)),
        block_agg_proof(roots[2], roots[3], Element::new(604)),
    ];

    let aggregated_messages = [Element::ZERO; 1000];
    let left_commit = hash_merge([
        block_proofs[0].public_inputs.commit_hash,
        block_proofs[1].public_inputs.commit_hash,
    ]);
    let right_commit = hash_merge([
        block_proofs[2].public_inputs.commit_hash,
        block_proofs[3].public_inputs.commit_hash,
    ]);
    let final_commit = hash_merge([left_commit, right_commit]);

    let aggregated_left_inputs = AggAggPublicInput {
        verification_key_hash: [Element::new(10), Element::new(11)],
        old_root: rolled_root,
        new_root: roots[1],
        commit_hash: left_commit,
        messages: aggregated_messages,
    };
    let aggregated_right_inputs = AggAggPublicInput {
        verification_key_hash: [Element::new(10), Element::new(11)],
        old_root: roots[1],
        new_root: roots[3],
        commit_hash: right_commit,
        messages: aggregated_messages,
    };
    let aggregated_final_inputs = AggAggPublicInput {
        verification_key_hash: [Element::new(10), Element::new(11)],
        old_root: rolled_root,
        new_root: roots[3],
        commit_hash: final_commit,
        messages: aggregated_messages,
    };
    let aggregated_left_inputs_ref = leak(aggregated_left_inputs.clone());
    let aggregated_right_inputs_ref = leak(aggregated_right_inputs.clone());
    let aggregated_final_inputs_ref = leak(aggregated_final_inputs.clone());

    let bb_backend: Arc<dyn BbBackend> = Arc::new(Unimock::new(()));

    let node_client: Arc<dyn NodeClient> = Arc::new(Unimock::new(()));
    let rollup_contract: Arc<dyn RollupContractTrait> = Arc::new(Unimock::new(()));
    let block_prover: Arc<dyn BlockProverTrait> = Arc::new(Unimock::new(()));
    let rollup_tree: Box<dyn RollupTree> = Box::new(Unimock::new(()));

    let agg_agg_proofs = [
        agg_agg_proof_from_inputs(&aggregated_left_inputs),
        agg_agg_proof_from_inputs(&aggregated_right_inputs),
        agg_agg_proof_from_inputs(&aggregated_final_inputs),
    ];
    let agg_agg_circuit: Arc<dyn AggAggCircuitInterface> = Arc::new(Unimock::new((
        CircuitMock::prove
            .with_types::<CircuitAggAggInput, _>()
            .some_call(matching!((agg_input, _)
                if agg_input.old_root == aggregated_left_inputs_ref.old_root
                    && agg_input.new_root == aggregated_left_inputs_ref.new_root))
            .returns(Ok::<_, CircuitError>(Proof::from(
                agg_agg_proofs[0].clone(),
            ))),
        CircuitMock::prove
            .with_types::<CircuitAggAggInput, _>()
            .some_call(matching!((agg_input, _)
                if agg_input.old_root == aggregated_right_inputs_ref.old_root
                    && agg_input.new_root == aggregated_right_inputs_ref.new_root))
            .returns(Ok::<_, CircuitError>(Proof::from(
                agg_agg_proofs[1].clone(),
            ))),
        CircuitMock::prove
            .with_types::<CircuitAggAggInput, _>()
            .some_call(matching!((agg_input, _)
                if agg_input.old_root == aggregated_final_inputs_ref.old_root
                    && agg_input.new_root == aggregated_final_inputs_ref.new_root))
            .returns(Ok::<_, CircuitError>(Proof::from(
                agg_agg_proofs[2].clone(),
            ))),
    )));
    let agg_final_circuit: Arc<dyn AggFinalCircuitInterface> = Arc::new(Unimock::new(()));
    let aggregator = Aggregator::new(
        node_client,
        rollup_contract,
        block_prover,
        rollup_tree,
        BLOCK_BATCH_SIZE,
        TEST_GAS_PER_BURN_CALL,
        agg_agg_circuit,
        agg_final_circuit,
    );

    let aggregated_proof = aggregator
        .aggregate_block_proofs(block_proofs, Arc::clone(&bb_backend))
        .await
        .expect("proof aggregation succeeds");

    assert_eq!(aggregated_proof.public_inputs.old_root, rolled_root);
    assert_eq!(aggregated_proof.public_inputs.new_root, roots[3]);
    assert_eq!(aggregated_proof.public_inputs.commit_hash, final_commit);
}
