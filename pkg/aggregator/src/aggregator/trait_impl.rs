use std::sync::Arc;

use aggregator_interface::{
    Aggregator as AggregatorTrait, ContractError, Error, PreparationOutcome, PreparedBatch,
    ProvenBatch, RollupInput,
};
use async_trait::async_trait;
use barretenberg_interface::BbBackend;
use contextful::{ErrorContextExt, ResultContextExt};
use futures::future::join_all;
use tracing::debug;
use zk_circuits::circuits::generated::agg_final::AggFinalInput as CircuitAggFinalInput;
use zk_primitives::AggFinal;

use crate::aggregator::{Aggregator, utils};

#[async_trait]
impl AggregatorTrait for Aggregator {
    async fn prepare_next_batch(&self) -> Result<PreparationOutcome, Error> {
        let tree = self.rollup_tree.lock().await;
        let start_height = tree.height() + 1;
        let expected_old_root = tree.root_hash();

        drop(tree);

        let mut blocks = self.fetch_blocks(start_height).await?;
        let available_blocks = blocks.len();
        if available_blocks < self.block_batch_size {
            return Ok(PreparationOutcome::InsufficientBlocks {
                start_height,
                available: available_blocks,
                required: self.block_batch_size,
            });
        }
        blocks.truncate(self.block_batch_size);

        let Some(last_block) = blocks.last() else {
            return Ok(PreparationOutcome::InsufficientBlocks {
                start_height,
                available: 0,
                required: self.block_batch_size,
            });
        };
        let new_height = last_block.block.content.header.height.0;

        let mut tree = self.rollup_tree.lock().await;

        if tree.height() + 1 != start_height || tree.root_hash() != expected_old_root {
            return Err(Error::from(
                ContractError::RootMismatch.wrap_err("local tree modified during preparation"),
            ));
        }

        let mut prepared_blocks = Vec::with_capacity(blocks.len());
        for block in &blocks {
            let block_height = block.block.content.header.height.0;
            debug!(block_height, "preparing block proof");
            let prepared = self
                .block_prover
                .prepare(block_height, tree.as_mut())
                .await
                .with_context(|| format!("prepare block proof for height {block_height}"))?;
            prepared_blocks.push(prepared);
        }

        let new_root = tree.root_hash();

        tree.set_height(new_height);

        Ok(PreparationOutcome::Success(PreparedBatch {
            blocks,
            prepared_blocks,
            old_root: expected_old_root,
            new_root,
        }))
    }

    async fn prove_batch(
        &self,
        batch: PreparedBatch,
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<ProvenBatch, Error> {
        let proof_results = join_all(batch.prepared_blocks.into_iter().map(|prepared| {
            let block_prover = Arc::clone(&self.block_prover);
            let height = prepared.height;
            let bb_backend = Arc::clone(&bb_backend);
            async move {
                block_prover
                    .prove(prepared, bb_backend)
                    .await
                    .with_context(|| format!("prove block proof for height {height}"))
            }
        }))
        .await;

        let mut proofs = Vec::with_capacity(batch.blocks.len());
        for proof in proof_results {
            proofs.push(proof?);
        }

        let aggregated_proof = self
            .aggregate_block_proofs(proofs, Arc::clone(&bb_backend))
            .await?;

        if aggregated_proof.public_inputs.old_root != batch.old_root {
            return Err(Error::from(
                ContractError::RootMismatch.wrap_err("aggregated proof root mismatch"),
            ));
        }

        let agg_final_input = CircuitAggFinalInput::from(AggFinal::new(aggregated_proof));
        let agg_final_circuit = Arc::clone(&self.agg_final_circuit);

        let final_proof = agg_final_circuit
            .prove(&agg_final_input, bb_backend)
            .await
            .map_err(|err| Error::ImplementationSpecific(Box::new(err)))?;

        let Some(final_block) = batch.blocks.last() else {
            return Err(Error::EmptyBatch);
        };
        let other_hash = utils::block_header_hash(&final_block.block.content.header)?;

        Ok(ProvenBatch {
            final_proof: final_proof.into(),
            blocks: batch.blocks,
            other_hash,
        })
    }

    async fn submit_batch(&self, batch: ProvenBatch) -> Result<(), Error> {
        let Some(final_block) = batch.blocks.last() else {
            return Err(Error::EmptyBatch);
        };
        let next_block_height = final_block.block.content.header.height.0 + 1;

        let next_block = self
            .fetch_block(next_block_height)
            .await?
            .ok_or(Error::MissingApprovalBlock)?;

        let rollup = RollupInput {
            proof: batch.final_proof.proof.0.clone(),
            old_root: batch.final_proof.public_inputs.old_root,
            new_root: batch.final_proof.public_inputs.new_root,
            commit_hash: batch.final_proof.public_inputs.commit_hash,
            utxo_messages: batch.final_proof.public_inputs.messages.clone(),
            kzg: batch.final_proof.kzg.clone(),
            other_hash: batch.other_hash,
            height: final_block.block.content.header.height.0,
            signatures: utils::serialize_signatures(&next_block.block.content.header.approvals),
            gas_per_burn_call: self.gas_per_burn_call,
        };

        self.rollup_contract
            .submit_rollup(&rollup)
            .await
            .context("submit rollup proof to contract")?;

        Ok(())
    }
}
