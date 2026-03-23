use std::sync::Arc;

use aggregator_interface::{BlockProverError, PreparedBlock, PreparedChunk, UTXO_AGGREGATIONS};
use barretenberg_interface::BbBackend;
use contextful::ResultContextExt;
use futures::future::join_all;
use zk_circuits::circuits::generated::agg_agg::AggAggInput as CircuitAggAggInput;
use zk_circuits::circuits::generated::agg_utxo::AggUtxoInput as CircuitAggUtxoInput;
use zk_primitives::{
    AggAgg, AggAggProof, AggProof, AggUtxo, AggUtxoProof, AggUtxoPublicInput, ProofBytes,
};

use crate::block_prover::{BlockProver, error::BlockProverImplError};

impl BlockProver {
    pub(super) async fn prove_impl(
        &self,
        prepared: PreparedBlock,
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<AggAggProof, BlockProverError> {
        let agg_proofs = self
            .prove_chunks(&prepared.chunks, Arc::clone(&bb_backend))
            .await?;
        let agg_agg = AggAgg::new(agg_proofs);
        let agg_agg_input = CircuitAggAggInput::from(agg_agg);
        let agg_agg_circuit = Arc::clone(&self.agg_agg_circuit);

        let proof = agg_agg_circuit
            .prove(&agg_agg_input, bb_backend)
            .await
            .context("prove agg agg rollup")
            .map_err(BlockProverImplError::from)?;
        let proof = AggAggProof::from(proof);

        Ok(proof)
    }

    pub(super) async fn prove_chunks(
        &self,
        chunks: &[PreparedChunk; UTXO_AGGREGATIONS],
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<[AggProof; UTXO_AGGREGATIONS], BlockProverError> {
        let chunk_futures = chunks
            .iter()
            .cloned()
            .map(|chunk| self.prove_chunk(chunk, Arc::clone(&bb_backend)));
        let results = join_all(chunk_futures).await;
        let mut proofs = Vec::with_capacity(UTXO_AGGREGATIONS);
        for result in results {
            proofs.push(result?);
        }

        let proof_len = proofs.len();
        proofs.try_into().map_err(|_| {
            BlockProverError::from(BlockProverImplError::ChunkCountMismatch {
                expected: UTXO_AGGREGATIONS,
                found: proof_len,
            })
        })
    }

    async fn prove_chunk(
        &self,
        chunk: PreparedChunk,
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<AggProof, BlockProverError> {
        if self.chunk_is_padding(&chunk) {
            return Ok(Self::padding_proof());
        }

        let agg_utxo = AggUtxo::new(chunk.bundles.clone(), chunk.old_root, chunk.new_root);
        let agg_utxo_input = CircuitAggUtxoInput::from(agg_utxo);
        let agg_utxo_circuit = Arc::clone(&self.agg_utxo_circuit);

        let proof = agg_utxo_circuit
            .prove(&agg_utxo_input, bb_backend)
            .await
            .context("prove agg utxo chunk")
            .map_err(BlockProverImplError::from)?;
        let proof = AggUtxoProof::from(proof);

        Ok(AggProof::AggUtxo(Box::new(proof)))
    }

    fn chunk_is_padding(&self, chunk: &PreparedChunk) -> bool {
        chunk
            .bundles
            .iter()
            .all(|bundle| bundle.utxo_proof.is_padding())
    }

    fn padding_proof() -> AggProof {
        AggProof::AggUtxo(Box::new(AggUtxoProof {
            proof: ProofBytes::default(),
            public_inputs: AggUtxoPublicInput::default(),
        }))
    }
}
