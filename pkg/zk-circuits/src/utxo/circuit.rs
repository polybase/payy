use crate::{
    chips::{
        add::{AddCulmChip, AddCulmChipConfig},
        binary_decomposition::BinaryDecompositionConfig,
        is_constant::{IsConstantChip, IsConstantConfig},
        poseidon::{P128Pow5T3Fr, PoseidonChip, PoseidonConfig},
        swap::{CondSwapChip, CondSwapConfig},
    },
    data::{Note, Utxo, UtxoKind},
};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    halo2curves::bn256::Fr,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};

#[derive(Clone, Debug)]
pub struct UtxoCircuitConfig {
    advices: [Column<Advice>; 5],
    instance: Column<Instance>,
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
    culm_add_config: AddCulmChipConfig,
    swap_config: CondSwapConfig,
    is_padding_config: IsConstantConfig<Fr>,
    is_mint_config: IsConstantConfig<Fr>,
    is_burn_config: IsConstantConfig<Fr>,
    binary_decomposition_config: BinaryDecompositionConfig<Fr, 1>,
}

impl<const MERKLE_D: usize> Circuit<Fr> for Utxo<MERKLE_D> {
    type FloorPlanner = SimpleFloorPlanner;
    type Config = UtxoCircuitConfig;

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

        // Is mint chip
        let is_mint_config = IsConstantChip::configure(
            meta,
            advices[0],
            advices[1],
            advices[2],
            UtxoKind::Mint.as_element(),
        );

        // Is burn chip
        let is_burn_config = IsConstantChip::configure(
            meta,
            advices[0],
            advices[1],
            advices[2],
            UtxoKind::Burn.as_element(),
        );

        let q_range_check = meta.selector();
        let binary_decomposition_config =
            BinaryDecompositionConfig::configure(meta, q_range_check, advices[0], advices[1]);

        UtxoCircuitConfig {
            advices,
            instance,
            poseidon_config,
            culm_add_config,
            swap_config,
            is_padding_config,
            is_mint_config,
            is_burn_config,
            binary_decomposition_config,
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
            config.instance,
            config.advices[0],
            config.poseidon_config,
            AddCulmChip::construct(config.culm_add_config),
            CondSwapChip::construct(config.swap_config),
            IsConstantChip::construct(config.is_padding_config),
            IsConstantChip::construct(config.is_mint_config),
            IsConstantChip::construct(config.is_burn_config),
            config.binary_decomposition_config,
        )?;

        Ok(())
    }
}
