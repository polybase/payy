use crate::{chips::{
    is_constant::{IsConstantChip, IsConstantConfig},
    poseidon::{P128Pow5T3Fr, PoseidonChip, PoseidonConfig},
    swap::{CondSwapChip, CondSwapConfig},
}, data::Mint};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    halo2curves::bn256::Fr,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};

#[derive(Clone, Debug)]
pub struct MintCircuitConfig {
    advices: [Column<Advice>; 5],
    instance: Column<Instance>,
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
    is_zero_config: IsConstantConfig<Fr>,
    swap_config: CondSwapConfig,
}

impl<const L: usize> Circuit<Fr> for Mint<L> {
    type FloorPlanner = SimpleFloorPlanner;
    type Config = MintCircuitConfig;

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
        ];
        meta.enable_constant(lagrange_coeffs[0]);

        let poseidon_config = PoseidonChip::configure::<P128Pow5T3Fr>(
            meta,
            advices[1..4].try_into().unwrap(),
            advices[0],
            lagrange_coeffs[0..3].try_into().unwrap(),
            lagrange_coeffs[3..6].try_into().unwrap(),
        );

        let swap_config = CondSwapChip::configure(meta, advices[0..5].try_into().unwrap());

        // Is zero chip
        let is_zero_config =
            IsConstantChip::configure(meta, advices[0], advices[1], advices[2], Fr::zero());

        //

        MintCircuitConfig {
            advices,
            instance,
            poseidon_config,
            is_zero_config,
            swap_config,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        self.enforce_constraints(
            layouter.namespace(|| "mint"),
            config.instance,
            config.advices[0],
            config.poseidon_config,
            IsConstantChip::construct(config.is_zero_config),
            CondSwapChip::construct(config.swap_config),
        )?;

        Ok(())
    }
}
