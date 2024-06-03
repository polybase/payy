use super::Compliance;
use crate::chips::{
    is_constant::{IsConstantChip, IsConstantConfig},
    poseidon::{P128Pow5T3Fr, PoseidonChip, PoseidonConfig},
    swap::{CondSwapChip, CondSwapConfig},
};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    halo2curves::bn256::Fr,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};

#[derive(Clone, Debug)]
pub struct ComplianceCircuitConfig {
    advices: [Column<Advice>; 5],
    instance: Column<Instance>,
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
    swap_config: CondSwapConfig,
    is_zero_config: IsConstantConfig<Fr>,
}

impl<const N: usize> Circuit<Fr> for Compliance<N> {
    type FloorPlanner = SimpleFloorPlanner;
    type Config = ComplianceCircuitConfig;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        let instance = meta.instance_column();
        meta.enable_equality(instance);

        let advices = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];

        for advice in advices.iter() {
            meta.enable_equality(*advice);
        }

        let lagrange_coeffs = [
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
        ];
        meta.enable_constant(lagrange_coeffs[0]);

        let poseidon_config = PoseidonChip::configure::<P128Pow5T3Fr>(
            meta,
            advices[0..3].try_into().unwrap(),
            advices[4],
            lagrange_coeffs[2..5].try_into().unwrap(),
            lagrange_coeffs[5..8].try_into().unwrap(),
        );

        let swap_config = CondSwapChip::configure(meta, advices[0..5].try_into().unwrap());

        // Zero chip
        let is_zero_config =
            IsConstantChip::configure(meta, advices[0], advices[1], advices[2], Fr::zero());

        ComplianceCircuitConfig {
            advices,
            instance,
            poseidon_config,
            swap_config,
            is_zero_config,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        // Get the public instances
        self.enforce_constraints(
            layouter.namespace(|| "compliance"),
            config.advices[0],
            config.instance,
            config.poseidon_config,
            CondSwapChip::construct(config.swap_config),
            IsConstantChip::construct(config.is_zero_config),
        )?;

        Ok(())
    }
}
