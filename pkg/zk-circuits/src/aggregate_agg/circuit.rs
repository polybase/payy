use crate::{chips::aggregation::aggregate::{
    AggregationChip, AggregationChipConfig, AggregationChipConfigParams,
}, data::AggregateAgg};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    halo2curves::bn256::Fr,
    plonk::{Circuit, Column, ConstraintSystem, Error, Instance},
};

#[derive(Clone, Debug)]
pub struct AggregateAggCircuitConfig {
    instance: Column<Instance>,
    aggregation_config: AggregationChipConfig,
}

impl<const AGG_N: usize> Circuit<Fr> for AggregateAgg<AGG_N> {
    type Config = AggregateAggCircuitConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        self.clone()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        let instance = meta.instance_column();
        meta.enable_equality(instance);

        let num_advice = 3 + AGG_N * 3;
        let num_lookup_advice = 1 + num_advice / 12;
        let params = AggregationChipConfigParams {
            strategy: halo2_ecc::fields::fp::FpStrategy::Simple,
            degree: 21,
            num_advice,
            num_lookup_advice,
            num_fixed: 1,
            lookup_bits: 20,
            limb_bits: 88,
            num_limbs: 3,
        };

        AggregateAggCircuitConfig {
            instance,
            aggregation_config: AggregationChip::configure(meta, params),
        }
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<Fr>) -> Result<(), Error> {
        // Build aggregation chip
        let aggregation_chip = AggregationChip::construct(config.aggregation_config);

        self.enforce_constraints(layouter, config.instance, &aggregation_chip)?;

        Ok(())
    }
}
