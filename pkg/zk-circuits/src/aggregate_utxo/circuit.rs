use super::aggregate::AggregateUtxo;
use crate::{
    chips::{
        aggregation::aggregate::{
            AggregationChip, AggregationChipConfig, AggregationChipConfigParams,
        },
        binary_decomposition::BinaryDecompositionConfig,
        is_constant::{IsConstantChip, IsConstantConfig},
        is_less_than::{IsLessThanChip, IsLessThanChipConfig},
        poseidon::{P128Pow5T3Fr, PoseidonChip, PoseidonConfig},
        swap::{CondSwapChip, CondSwapConfig},
    },
    data::Note,
};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    halo2curves::bn256::Fr,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};

#[derive(Clone, Debug)]
pub struct AggregateUtxoCircuitConfig {
    instance: Column<Instance>,
    advice: Column<Advice>,
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
    binary_decomposition_config: BinaryDecompositionConfig<Fr, 1>,
    swap_config: CondSwapConfig,
    is_padding_config: IsConstantConfig<Fr>,
    aggregation_config: AggregationChipConfig,
    is_less_than: IsLessThanChipConfig,
}

impl<const UTXO_N: usize, const MERKLE_D: usize, const LEAVES: usize> Circuit<Fr>
    for AggregateUtxo<UTXO_N, MERKLE_D, LEAVES>
{
    type Config = AggregateUtxoCircuitConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        self.clone()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        let instance = meta.instance_column();
        meta.enable_equality(instance);

        let num_advice = 2 + UTXO_N * 2;
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

        let aggregation_config = AggregationChip::configure(meta, params);

        // let advices = aggregation_config
        //     .base_field_config
        //     .range
        //     .gate
        //     .basic_gates
        //     .iter()
        //     .flat_map(|gate| gate.iter().map(|gate| gate.value))
        //     .collect::<Vec<_>>();

        // println!("advices: {:?}", advices.len());

        // 2 advices
        // let advices = aggregation_config
        //     .base_field_config
        //     .range
        //     .lookup_advice
        //     .iter()
        //     .flatten()
        //     .cloned()
        //     .collect::<Vec<_>>();

        // println!("advices: {:?}", advices.len());

        // // 1 fixed
        // let fixed_1 = aggregation_config
        //     .base_field_config
        //     .range
        //     .gate
        //     .constants
        //     .iter()
        //     .collect::<Vec<_>>();

        // println!("fixed: {:?}", fixed_1.len());

        // // x fixed
        // let fixed_2 = aggregation_config
        //     .base_field_config
        //     .range
        //     .gate
        //     .basic_gates
        //     .iter()
        //     .flat_map(|gate| gate.iter().flat_map(|gate| gate.q_enable_plus.clone()))
        //     .collect::<Vec<_>>();

        // println!("fixed: {:?}", fixed_2.len());

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
            // *fixed_1[0],
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

        let q_range_check = meta.selector();

        // TODO: q_range_check doesn't seem corrrect
        let binary_decomposition_config =
            BinaryDecompositionConfig::configure(meta, q_range_check, advices[0], advices[1]);

        let swap_config = CondSwapChip::configure(meta, advices[0..5].try_into().unwrap());

        // Padding chip
        let is_padding_config = IsConstantChip::configure(
            meta,
            advices[0],
            advices[1],
            advices[2],
            Note::padding_note().commitment().into(),
        );

        let is_less_than =
            IsLessThanChip::configure(meta, [advices[0], advices[1], advices[2], advices[3]]);

        AggregateUtxoCircuitConfig {
            instance,
            advice: advices[0],
            poseidon_config,
            binary_decomposition_config,
            swap_config,
            is_padding_config,
            aggregation_config,
            is_less_than,
        }
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<Fr>) -> Result<(), Error> {
        // Build aggregation chip
        let aggregation_chip = AggregationChip::construct(config.aggregation_config);

        self.enforce_constraints(
            layouter,
            config.instance,
            config.advice,
            config.poseidon_config,
            config.binary_decomposition_config,
            CondSwapChip::construct(config.swap_config),
            IsConstantChip::construct(config.is_padding_config),
            &aggregation_chip,
            &IsLessThanChip::construct(config.is_less_than),
        )?;

        Ok(())
    }
}
