// lint-long-file-override allow-max-lines=300
use super::BlockProver;
use aggregator_interface::{
    BlockProver as BlockProverTrait, PreparedBlock, PreparedChunk, RollupTreeMock,
};
use barretenberg_interface::BbBackend;
use element::Element;
use node_interface::{
    Block, BlockContent, BlockHeader, BlockState, BlockTreeDiff, BlockTreeDiffChanges,
    BlockWithInfo, ListBlocksResponse, NodeClient, NodeClientMock, TxnWithInfo,
};
use primitives::{
    block_height::BlockHeight, hash::CryptoHash, pagination::OpaqueCursor, sig::Signature,
};
use std::sync::Arc;
use unimock::{MockFn, Unimock, matching};
use zk_circuits::circuits::generated::agg_agg::AggAggInput as CircuitAggAggInput;
use zk_circuits::{Error as CircuitError, Proof};
use zk_primitives::{
    AggAggProof, AggAggPublicInput, ProofBytes, UtxoProof, UtxoProofBundleWithMerkleProofs,
};

use crate::CircuitMock;

fn sample_block(height: u64, txns: Vec<UtxoProof>, root_hash: Element) -> BlockWithInfo {
    let txns = txns
        .into_iter()
        .enumerate()
        .map(|(idx, proof)| TxnWithInfo {
            proof,
            hash: Element::new(idx as u64),
            index_in_block: idx as u64,
            block_height: BlockHeight(height),
            time: height,
        })
        .collect();
    BlockWithInfo {
        block: Block {
            content: BlockContent {
                header: BlockHeader {
                    height: BlockHeight(height),
                    last_block_hash: CryptoHash::from_u64(height - 1),
                    epoch_id: 0,
                    last_final_block_hash: CryptoHash::from_u64(height - 2),
                    approvals: vec![Signature([1; 65])],
                },
                state: BlockState { root_hash, txns },
            },
            signature: Signature::default(),
        },
        hash: CryptoHash::from_u64(height * 100),
        time: height,
    }
}

fn sample_diff(height: u64, root_hash: Element) -> BlockTreeDiff {
    BlockTreeDiff {
        height: BlockHeight(height),
        root_hash,
        diff: BlockTreeDiffChanges {
            from_height: BlockHeight(height - 1),
            additions: vec![Element::new(height * 10 + 1)],
            removals: vec![Element::new(height * 10 + 2)],
        },
    }
}

#[tokio::test]
async fn prepare_fetches_block_tree_and_block() {
    let prove_height = leak(42u64);
    let blocks_response = ListBlocksResponse {
        blocks: vec![sample_block(*prove_height, Vec::new(), Element::ZERO)],
        cursor: OpaqueCursor::default(),
    };

    let node_client: Arc<dyn NodeClient> = Arc::new(Unimock::new((
        NodeClientMock::block_tree_diff
            .next_call(matching!((height, diff_from)
                if *height == BlockHeight(*prove_height)
                    && *diff_from == BlockHeight(*prove_height - 1)))
            .returns(Ok(sample_diff(*prove_height, Element::ZERO))),
        NodeClientMock::blocks
            .next_call(matching!((start_height, limit, skip_empty)
                if *start_height == BlockHeight(*prove_height)
                    && *limit == 1
                    && !*skip_empty))
            .returns(Ok(blocks_response)),
    )));

    let mut tree = Unimock::new((
        RollupTreeMock::height
            .next_call(matching!(()))
            .returns(*prove_height - 1),
        RollupTreeMock::set_height
            .next_call(matching!((height) if *height == *prove_height))
            .returns(()),
        RollupTreeMock::root_hash
            .each_call(matching!(()))
            .returns(Element::ZERO),
    ));

    let agg_utxo_circuit = Arc::new(Unimock::new(()));
    let agg_agg_circuit = Arc::new(Unimock::new(()));
    let prover = BlockProver::new(node_client, agg_utxo_circuit, agg_agg_circuit);
    let _ = prover.prepare(*prove_height, &mut tree).await;
}

#[tokio::test]
async fn prove_succeeds_with_padding_chunks() {
    let node_client: Arc<dyn NodeClient> = Arc::new(Unimock::new(()));
    let bb_backend: Arc<dyn BbBackend> = Arc::new(Unimock::new(()));
    let agg_utxo_circuit = Arc::new(Unimock::new(()));
    let agg_agg_proof = AggAggProof {
        proof: ProofBytes::default(),
        public_inputs: AggAggPublicInput {
            verification_key_hash: [Element::ZERO; 2],
            old_root: Element::ZERO,
            new_root: Element::ZERO,
            commit_hash: Element::ZERO,
            messages: [Element::ZERO; 1000],
        },
        kzg: vec![],
    };
    let agg_agg_circuit = Arc::new(Unimock::new(
        CircuitMock::prove
            .with_types::<CircuitAggAggInput, _>()
            .next_call(matching!((_, _)))
            .returns(Ok::<_, CircuitError>(Proof::from(agg_agg_proof.clone()))),
    ));
    let prover = BlockProver::new(node_client, agg_utxo_circuit, agg_agg_circuit);
    let prepared = PreparedBlock {
        height: 7,
        chunks: [padding_chunk(), padding_chunk()],
    };

    let proof = prover.prove(prepared, bb_backend.clone()).await.unwrap();
    assert_eq!(proof.public_inputs.old_root, Element::ZERO);
}

fn padding_chunk() -> PreparedChunk {
    PreparedChunk {
        old_root: Element::ZERO,
        new_root: Element::ZERO,
        bundles: [
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
    }
}

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}
