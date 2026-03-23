use contextful::ResultContextExt as _;
use prover::smirk_metadata::SmirkMetadata;
use smirk::Batch;
use tracing::instrument;

use crate::{
    Error, NodeShared, PersistentMerkleTree, Result,
    block::{Block, BlockState},
    types::BlockHeight,
};

impl NodeShared {
    #[instrument(skip_all)]
    pub(super) async fn validate_block(&self, block: &Block) -> Result<()> {
        if self
            .config
            .bad_blocks
            .contains(&block.content.header.height)
        {
            return Ok(());
        }

        let validator = self.get_leader_for_block_height(block.content.header.height);

        let signed_by = block
            .signature
            .verify(&block.hash())
            .ok_or(Error::InvalidSignature)?;

        if signed_by != validator {
            return Err(Error::InvalidSignature);
        }

        block
            .content
            .validate(
                &*self.bb_backend,
                self.config.mode,
                &self.block_store,
                &self.notes_tree,
            )
            .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) fn apply_block_to_tree(
        notes_tree: &mut PersistentMerkleTree,
        state: &BlockState,
        current_height: BlockHeight,
    ) -> Result<()> {
        let insert_leaves = state
            .txns
            .iter()
            .flat_map(|txn| txn.public_inputs.output_commitments)
            .filter(|e| !e.is_zero());

        let remove_leaves = state
            .txns
            .iter()
            .flat_map(|txn| txn.public_inputs.input_commitments)
            .filter(|e| !e.is_zero());

        let metadata = SmirkMetadata::inserted_in(current_height.0);
        let leaves_with_height = insert_leaves.map(|e| (e, metadata.clone()));
        let batch = Batch::from_entries(leaves_with_height, remove_leaves.collect::<Vec<_>>())
            .context("construct smirk batch for block application")?;

        notes_tree
            .insert_batch(batch)
            .context("apply smirk batch to persistent notes tree")?;
        Ok(())
    }
}
