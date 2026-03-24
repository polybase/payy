use std::sync::Arc;

use aggregator_interface::{
    BlockProver as BlockProverTrait, BlockProverError, PreparedBlock, RollupTree,
};
use async_trait::async_trait;
use barretenberg_interface::BbBackend;
use node_interface::NodeClient;
use zk_primitives::AggAggProof;

pub mod error;
pub mod prepare;
pub mod prove;

#[cfg(test)]
mod tests;

use crate::{AggAggCircuitInterface, AggUtxoCircuitInterface};

#[derive(Clone)]
pub struct BlockProver {
    pub(crate) node_client: Arc<dyn NodeClient>,
    pub(crate) agg_utxo_circuit: Arc<dyn AggUtxoCircuitInterface>,
    pub(crate) agg_agg_circuit: Arc<dyn AggAggCircuitInterface>,
}

impl BlockProver {
    pub fn new(
        node_client: Arc<dyn NodeClient>,
        agg_utxo_circuit: Arc<dyn AggUtxoCircuitInterface>,
        agg_agg_circuit: Arc<dyn AggAggCircuitInterface>,
    ) -> Self {
        Self {
            node_client,
            agg_utxo_circuit,
            agg_agg_circuit,
        }
    }
}

#[async_trait]
impl BlockProverTrait for BlockProver {
    async fn prepare(
        &self,
        height: u64,
        tree: &mut dyn RollupTree,
    ) -> Result<PreparedBlock, BlockProverError> {
        self.prepare_impl(height, tree).await
    }

    async fn prove(
        &self,
        prepared: PreparedBlock,
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<AggAggProof, BlockProverError> {
        self.prove_impl(prepared, bb_backend).await
    }
}
