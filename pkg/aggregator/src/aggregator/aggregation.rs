use std::{io, sync::Arc};

use aggregator_interface::Error;
use barretenberg_interface::BbBackend;
use futures::future::join_all;
use zk_circuits::circuits::generated::agg_agg::AggAggInput as CircuitAggAggInput;
use zk_primitives::{AggAgg, AggAggProof, AggProof};

use crate::aggregator::Aggregator;

impl Aggregator {
    pub(super) async fn aggregate_block_proofs(
        &self,
        proofs: Vec<AggAggProof>,
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<AggAggProof, Error> {
        if proofs.len() != self.block_batch_size {
            return Err(Error::ImplementationSpecific(Box::new(io::Error::other(
                format!(
                    "expected {} block proofs, got {}",
                    self.block_batch_size,
                    proofs.len()
                ),
            ))));
        }

        let mut level: Vec<AggProof> = proofs
            .into_iter()
            .map(|proof| AggProof::AggAgg(Box::new(proof)))
            .collect();

        while level.len() > 1 {
            if !level.len().is_multiple_of(2) {
                return Err(Error::ImplementationSpecific(Box::new(io::Error::other(
                    format!(
                        "aggregation level requires an even number of proofs, got {}",
                        level.len()
                    ),
                ))));
            }

            let mut futures = Vec::with_capacity(level.len() / 2);
            let mut iter = level.into_iter();
            while let (Some(left), Some(right)) = (iter.next(), iter.next()) {
                let aggregated = AggAgg::new([left, right]);
                let aggregated_input = CircuitAggAggInput::from(aggregated);
                let agg_agg_circuit = Arc::clone(&self.agg_agg_circuit);
                let bb_backend = Arc::clone(&bb_backend);

                futures.push(async move {
                    let proof = agg_agg_circuit
                        .prove(&aggregated_input, bb_backend)
                        .await
                        .map_err(|err| Error::ImplementationSpecific(Box::new(err)))?;
                    Ok::<AggAggProof, Error>(AggAggProof::from(proof))
                });
            }

            let results = join_all(futures).await;
            let mut next_level = Vec::with_capacity(results.len());
            for result in results {
                let proof = result?;
                next_level.push(AggProof::AggAgg(Box::new(proof)));
            }

            level = next_level;
        }

        match level.pop() {
            Some(AggProof::AggAgg(proof)) => Ok(*proof),
            Some(AggProof::AggUtxo(_)) => Err(Error::ImplementationSpecific(Box::new(
                io::Error::other("unexpected agg_utxo proof"),
            ))),
            None => Err(Error::ImplementationSpecific(Box::new(io::Error::other(
                "missing aggregated proof",
            )))),
        }
    }
}
