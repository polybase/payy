// lint-long-file-override allow-max-lines=300
use std::array;

use aggregator_interface::{
    BlockProverError, PreparedBlock, PreparedChunk, RollupTree, UTXO_AGG_NUMBER, UTXO_AGGREGATIONS,
};
use contextful::ResultContextExt;
use element::Element;
use primitives::block_height::BlockHeight;
use zk_primitives::{MerklePath, UtxoProof, UtxoProofBundleWithMerkleProofs};

use crate::block_prover::{BlockProver, error::BlockProverImplError};

pub const MERKLE_TREE_DEPTH: usize = 161;
pub const MERKLE_TREE_PATH_DEPTH: usize = MERKLE_TREE_DEPTH - 1;
pub const MAXIMUM_TXNS: usize = UTXO_AGG_NUMBER * UTXO_AGGREGATIONS;

impl BlockProver {
    pub(super) async fn prepare_impl(
        &self,
        height: u64,
        tree: &mut dyn RollupTree,
    ) -> Result<PreparedBlock, BlockProverError> {
        let previous_height = tree.height();
        let diff = self
            .node_client
            .block_tree_diff(BlockHeight(height), BlockHeight(previous_height))
            .await
            .context("fetch block tree diff")
            .map_err(BlockProverImplError::from)?;
        self.validate_diff(&diff, height, previous_height)?;

        let blocks = self
            .node_client
            .blocks(BlockHeight(height), 1, false)
            .await
            .context("fetch blocks from node client")
            .map_err(BlockProverImplError::from)?;
        let block = blocks
            .blocks
            .first()
            .ok_or(BlockProverImplError::MissingBlock(height))?;
        let block_height = block.block.content.header.height.0;
        if block_height != height {
            return Err(BlockProverImplError::BlockHeightMismatch {
                expected: height,
                found: block_height,
            }
            .into());
        }

        let mut proofs = block
            .block
            .content
            .state
            .txns
            .iter()
            .map(|txn| Some(txn.proof.clone()))
            .collect::<Vec<_>>();
        if proofs.len() > MAXIMUM_TXNS {
            return Err(BlockProverImplError::TooManyTransactions {
                found: proofs.len(),
                max: MAXIMUM_TXNS,
            }
            .into());
        }
        while proofs.len() < MAXIMUM_TXNS {
            proofs.push(None);
        }

        let mut prepared_chunks = Vec::with_capacity(UTXO_AGGREGATIONS);
        for chunk in proofs.chunks(UTXO_AGG_NUMBER) {
            let chunk_array = [chunk[0].clone(), chunk[1].clone(), chunk[2].clone()];
            let prepared_chunk = self.build_chunk(tree, chunk_array, height)?;
            prepared_chunks.push(prepared_chunk);
        }

        if prepared_chunks.len() != UTXO_AGGREGATIONS {
            return Err(BlockProverImplError::ChunkCountMismatch {
                expected: UTXO_AGGREGATIONS,
                found: prepared_chunks.len(),
            }
            .into());
        }

        let expected_root = block.block.content.state.root_hash;
        let final_root = tree.root_hash();
        if final_root != expected_root {
            return Err(BlockProverImplError::RootMismatch {
                expected: expected_root,
                got: final_root,
            }
            .into());
        }
        if final_root != diff.root_hash {
            return Err(BlockProverImplError::RootMismatch {
                expected: diff.root_hash,
                got: final_root,
            }
            .into());
        }

        tree.set_height(height);

        let chunk_len = prepared_chunks.len();
        let chunks: [PreparedChunk; UTXO_AGGREGATIONS] =
            prepared_chunks
                .try_into()
                .map_err(|_| BlockProverImplError::ChunkCountMismatch {
                    expected: UTXO_AGGREGATIONS,
                    found: chunk_len,
                })?;

        Ok(PreparedBlock { height, chunks })
    }

    fn validate_diff(
        &self,
        diff: &node_interface::BlockTreeDiff,
        height: u64,
        previous_height: u64,
    ) -> Result<(), BlockProverError> {
        if diff.height.0 != height {
            return Err(BlockProverImplError::DiffHeightMismatch {
                expected: height,
                found: diff.height.0,
            }
            .into());
        }
        let from_height = diff.diff.from_height.0;
        if from_height != previous_height {
            return Err(BlockProverImplError::DiffFromMismatch {
                expected: previous_height,
                found: from_height,
            }
            .into());
        }
        Ok(())
    }

    fn build_chunk(
        &self,
        tree: &mut dyn RollupTree,
        proofs: [Option<UtxoProof>; UTXO_AGG_NUMBER],
        height: u64,
    ) -> Result<PreparedChunk, BlockProverError> {
        let chunk_old_root = tree.root_hash();
        let mut bundles = Vec::with_capacity(UTXO_AGG_NUMBER);
        for proof in proofs.into_iter() {
            let bundle = match proof {
                Some(proof) => self.build_bundle(tree, proof, height)?,
                None => UtxoProofBundleWithMerkleProofs::default(),
            };
            bundles.push(bundle);
        }
        let chunk_new_root = tree.root_hash();
        let bundle_len = bundles.len();
        let bundles: [UtxoProofBundleWithMerkleProofs; UTXO_AGG_NUMBER] = bundles
            .try_into()
            .map_err(|_| BlockProverImplError::BundleCountMismatch {
                expected: UTXO_AGG_NUMBER,
                found: bundle_len,
            })?;

        Ok(PreparedChunk {
            old_root: chunk_old_root,
            new_root: chunk_new_root,
            bundles,
        })
    }

    fn build_bundle(
        &self,
        tree: &mut dyn RollupTree,
        proof: UtxoProof,
        height: u64,
    ) -> Result<UtxoProofBundleWithMerkleProofs, BlockProverError> {
        let mut merkle_paths = array::from_fn(|_| MerklePath::default());
        for (idx, commitment) in proof.public_inputs.input_commitments.iter().enumerate() {
            merkle_paths[idx] = self.extract_input_path(tree, *commitment)?;
        }

        for (idx, commitment) in proof.public_inputs.output_commitments.iter().enumerate() {
            merkle_paths[2 + idx] = self.extract_output_path(tree, *commitment, height)?;
        }

        Ok(UtxoProofBundleWithMerkleProofs::new(proof, &merkle_paths))
    }

    fn extract_input_path(
        &self,
        tree: &mut dyn RollupTree,
        commitment: Element,
    ) -> Result<MerklePath<MERKLE_TREE_DEPTH>, BlockProverError> {
        if commitment.is_zero() {
            return Ok(MerklePath::default());
        }
        let path_vec = tree.sibling_path(commitment)?;
        let merkle_path = self.path_from_vec(path_vec)?;
        tree.remove(commitment)?;
        Ok(merkle_path)
    }

    fn extract_output_path(
        &self,
        tree: &mut dyn RollupTree,
        commitment: Element,
        height: u64,
    ) -> Result<MerklePath<MERKLE_TREE_DEPTH>, BlockProverError> {
        if commitment.is_zero() {
            return Ok(MerklePath::default());
        }
        tree.insert(&[(commitment, height)])?;
        let path_vec = tree.sibling_path(commitment)?;
        self.path_from_vec(path_vec)
    }

    fn path_from_vec(
        &self,
        path: Vec<Element>,
    ) -> Result<MerklePath<MERKLE_TREE_DEPTH>, BlockProverError> {
        if path.len() != MERKLE_TREE_PATH_DEPTH {
            return Err(BlockProverImplError::MerklePathLength {
                expected: MERKLE_TREE_PATH_DEPTH,
                found: path.len(),
            }
            .into());
        }
        Ok(MerklePath::new(path))
    }
}
