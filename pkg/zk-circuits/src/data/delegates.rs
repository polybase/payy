use crate::{aggregate_utxo::AggregateUtxoPrivate, CircuitKind};

use super::{AggregateUtxo, SnarkWitnessV1};

impl<const UTXO_N: usize, const MERKLE_D: usize, const LEAVES: usize>
    AggregateUtxo<UTXO_N, MERKLE_D, LEAVES>
{
    pub fn snark(&self) -> SnarkWitnessV1 {
        let private = AggregateUtxoPrivate::new(self.clone());
        private.snark().unwrap().to_witness()
    }
}
