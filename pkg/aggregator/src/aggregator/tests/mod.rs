use super::*;
use aggregator_interface::{PreparedChunk, UTXO_AGG_NUMBER};
use element::Element;
use node_interface::{Block, BlockContent, BlockHeader, BlockState, BlockWithInfo, TxnWithInfo};
pub use primitives::block_height::BlockHeight;
use primitives::{hash::CryptoHash, sig::Signature};
use std::array;
use zk_primitives::{
    AggAggProof, AggAggPublicInput, ProofBytes, UtxoProof, UtxoProofBundleWithMerkleProofs,
    UtxoPublicInput,
};

pub mod aggregation;
pub mod failures;
pub mod fetch;
pub mod lifecycle;

pub const GAS_PER_BURN_CALL: u128 = 1_000_000;
pub const BYTES_PER_ELEMENT: usize = 32;
pub const AGG_FINAL_PROOF_ELEMENT_COUNT: usize = 330;
pub const DEFAULT_BLOCK_BATCH_SIZE: usize = 2;
pub const TEST_GAS_PER_BURN_CALL: u128 = 1_000_000;

pub fn block_with_single_txn(
    height: u64,
    root_hash: Element,
    approvals: Vec<Signature>,
    hash_seed: u64,
) -> BlockWithInfo {
    BlockWithInfo {
        block: Block {
            content: BlockContent {
                header: BlockHeader {
                    height: BlockHeight(height),
                    last_block_hash: CryptoHash::from_u64(height - 1),
                    epoch_id: 0,
                    last_final_block_hash: CryptoHash::from_u64(height - 2),
                    approvals,
                },
                state: BlockState {
                    root_hash,
                    txns: vec![txn(height)],
                },
            },
            signature: Signature::default(),
        },
        hash: CryptoHash::from_u64(hash_seed),
        time: height,
    }
}

pub fn txn(block_height: u64) -> TxnWithInfo {
    TxnWithInfo {
        proof: UtxoProof {
            proof: ProofBytes::default(),
            public_inputs: UtxoPublicInput {
                input_commitments: [Element::new(block_height), Element::new(block_height + 1)],
                output_commitments: [
                    Element::new(block_height + 2),
                    Element::new(block_height + 3),
                ],
                messages: [
                    Element::new(block_height + 4),
                    Element::new(block_height + 5),
                    Element::new(block_height + 6),
                    Element::new(block_height + 7),
                    Element::new(block_height + 8),
                ],
            },
        },
        hash: Element::new(block_height * 10),
        index_in_block: 0,
        block_height: BlockHeight(block_height),
        time: block_height,
    }
}

pub fn block_agg_proof(old_root: Element, new_root: Element, commit_hash: Element) -> AggAggProof {
    AggAggProof {
        proof: ProofBytes::default(),
        public_inputs: AggAggPublicInput {
            verification_key_hash: [Element::new(1), Element::new(2)],
            old_root,
            new_root,
            commit_hash,
            messages: [Element::ZERO; 1000],
        },
        kzg: vec![],
    }
}

pub fn agg_agg_proof_from_inputs(inputs: &AggAggPublicInput) -> AggAggProof {
    AggAggProof {
        proof: ProofBytes::default(),
        public_inputs: inputs.clone(),
        kzg: vec![],
    }
}

pub fn signature(byte: u8) -> Signature {
    Signature([byte; 65])
}

pub fn dummy_chunk(old_root: Element, new_root: Element) -> PreparedChunk {
    let bundles: [UtxoProofBundleWithMerkleProofs; UTXO_AGG_NUMBER] =
        array::from_fn(|_| UtxoProofBundleWithMerkleProofs::default());
    PreparedChunk {
        old_root,
        new_root,
        bundles,
    }
}

pub fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}
