use crate::{
    chips::{
        add::{AddCulmChip, AddCulmChipConfig},
        is_constant::{IsConstantChip, IsConstantConfig},
        poseidon::{P128Pow5T3Fr, PoseidonChip, PoseidonConfig},
        swap::{CondSwapChip, CondSwapConfig},
    },
    data::{Note, Points},
};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    halo2curves::bn256::Fr,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};

#[derive(Clone, Debug)]
pub struct PointsCircuitConfig {
    advices: [Column<Advice>; 5],
    instance: Column<Instance>,
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
    culm_add_config: AddCulmChipConfig,
    swap_config: CondSwapConfig,
    is_padding_config: IsConstantConfig<Fr>,
}

impl Circuit<Fr> for Points {
    type FloorPlanner = SimpleFloorPlanner;
    type Config = PointsCircuitConfig;

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

        let culm_add_config = AddCulmChip::configure(meta, advices[0], advices[1]);

        let swap_config = CondSwapChip::configure(meta, advices[0..5].try_into().unwrap());

        // Padding chip
        let is_padding_config = IsConstantChip::configure(
            meta,
            advices[0],
            advices[1],
            advices[2],
            Note::padding_note().commitment().into(),
        );

        PointsCircuitConfig {
            advices,
            instance,
            poseidon_config,
            culm_add_config,
            swap_config,
            is_padding_config,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        // Get the public instances
        self.enforce_constraints(
            layouter.namespace(|| "txn"),
            config.advices[0],
            config.instance,
            AddCulmChip::construct(config.culm_add_config),
            config.poseidon_config,
            CondSwapChip::construct(config.swap_config),
            IsConstantChip::construct(config.is_padding_config),
        )?;

        Ok(())
    }
}
