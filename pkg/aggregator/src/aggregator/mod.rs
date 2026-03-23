use std::sync::Arc;

use aggregator_interface::{BlockProver, RollupContract, RollupTree};
use node_interface::NodeClient;
use tokio::sync::Mutex;

pub mod aggregation;
pub mod fetch;
pub mod trait_impl;
pub mod utils;

#[cfg(test)]
mod tests;

use crate::{AggAggCircuitInterface, AggFinalCircuitInterface};

#[derive(Clone)]
pub struct Aggregator {
    pub(crate) node_client: Arc<dyn NodeClient>,
    pub(crate) rollup_contract: Arc<dyn RollupContract>,
    pub(crate) block_prover: Arc<dyn BlockProver>,
    pub(crate) rollup_tree: Arc<Mutex<Box<dyn RollupTree>>>,
    pub(crate) block_batch_size: usize,
    pub(crate) gas_per_burn_call: u128,
    pub(crate) agg_agg_circuit: Arc<dyn AggAggCircuitInterface>,
    pub(crate) agg_final_circuit: Arc<dyn AggFinalCircuitInterface>,
}

impl Aggregator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_client: Arc<dyn NodeClient>,
        rollup_contract: Arc<dyn RollupContract>,
        block_prover: Arc<dyn BlockProver>,
        rollup_tree: Box<dyn RollupTree>,
        block_batch_size: usize,
        gas_per_burn_call: u128,
        agg_agg_circuit: Arc<dyn AggAggCircuitInterface>,
        agg_final_circuit: Arc<dyn AggFinalCircuitInterface>,
    ) -> Self {
        assert!(
            block_batch_size >= 2 && block_batch_size.is_power_of_two(),
            "block_batch_size must be a power-of-two >= 2"
        );
        assert!(gas_per_burn_call > 0, "gas_per_burn_call must be > 0");

        Self {
            node_client,
            rollup_contract,
            block_prover,
            rollup_tree: Arc::new(Mutex::new(rollup_tree)),
            block_batch_size,
            gas_per_burn_call,
            agg_agg_circuit,
            agg_final_circuit,
        }
    }
}
