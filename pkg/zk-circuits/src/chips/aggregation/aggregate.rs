use super::snark::Snark;
use halo2_base::halo2_proofs::halo2curves::bn256::{Fq, Fr, G1Affine};
use halo2_base::halo2_proofs::plonk::Error;
use halo2_base::halo2_proofs::{
    circuit::{Cell, Layouter, Value},
    plonk::ConstraintSystem,
};

use super::constants::{BITS, LIMBS};
use super::types::{As, Halo2Loader, Plonk, PoseidonTranscript, SnarkInstanceColumnCells};

use halo2_base::{Context, ContextParams};
use halo2_ecc::ecc::EccChip;
use itertools::Itertools;
use rand::rngs::OsRng;
use snark_verifier::{
    loader::native::NativeLoader,
    pcs::{kzg::KzgAccumulator, AccumulationScheme, AccumulationSchemeProver},
    util::arithmetic::fe_to_limbs,
    verifier::PlonkVerifier,
};
use std::rc::Rc;

#[derive(Clone)]
pub struct AggregationChipConfigParams {
    pub strategy: halo2_ecc::fields::fp::FpStrategy,
    pub degree: u32,
    pub num_advice: usize,
    pub num_lookup_advice: usize,
    pub num_fixed: usize,
    pub lookup_bits: usize,
    pub limb_bits: usize,
    pub num_limbs: usize,
}

#[derive(Clone, Debug)]
pub struct AggregationChipConfig {
    pub base_field_config: halo2_ecc::fields::fp::FpConfig<Fr, Fq>,
}

impl AggregationChipConfig {
    pub fn configure(meta: &mut ConstraintSystem<Fr>, params: AggregationChipConfigParams) -> Self {
        assert!(
            params.limb_bits == BITS && params.num_limbs == LIMBS,
            "For now we fix limb_bits = {BITS}, otherwise change code"
        );

        let base_field_config = halo2_ecc::fields::fp::FpConfig::configure(
            meta,
            params.strategy,
            &[params.num_advice],
            &[params.num_lookup_advice],
            params.num_fixed,
            params.lookup_bits,
            params.limb_bits,
            params.num_limbs,
            halo2_base::utils::modulus::<Fq>(),
            0,
            params.degree as usize,
        );

        Self { base_field_config }
    }

    pub fn range(&self) -> &halo2_base::gates::range::RangeConfig<Fr> {
        &self.base_field_config.range
    }

    pub fn ecc_chip(&self) -> halo2_ecc::ecc::BaseFieldEccChip<G1Affine> {
        EccChip::construct(self.base_field_config.clone())
    }
}

#[derive(Clone)]
pub struct AggregationChip {
    config: AggregationChipConfig,
}

impl AggregationChip {
    pub fn construct(config: AggregationChipConfig) -> Self {
        Self { config }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<Fr>,
        params: AggregationChipConfigParams,
    ) -> AggregationChipConfig {
        AggregationChipConfig::configure(meta, params)
    }

    pub fn aggregate(
        &self,
        mut layouter: impl Layouter<Fr>,
        snarks: &[&Snark],
        as_proof: Value<&[u8]>,
    ) -> Result<(Vec<Cell>, Vec<Vec<SnarkInstanceColumnCells>>), Error> {
        self.config.range().load_lookup_table(&mut layouter)?;
        let max_rows = self.config.range().gate.max_rows;

        layouter.assign_region(
            || "",
            |region| {
                let ctx = Context::new(
                    region,
                    ContextParams {
                        max_rows,
                        num_context_ids: 1,
                        fixed_columns: self.config.base_field_config.range.gate.constants.clone(),
                    },
                );

                let ecc_chip = self.config.ecc_chip();
                let loader = Halo2Loader::new(ecc_chip, ctx);
                let (KzgAccumulator { lhs, rhs }, instances) =
                    accumulator_ecc(&loader, snarks, as_proof);

                let lhs = lhs.assigned();
                let rhs = rhs.assigned();

                self.config
                    .base_field_config
                    .finalize(&mut loader.ctx_mut());

                let agg_instances: Vec<_> = lhs
                    .x
                    .truncation
                    .limbs
                    .iter()
                    .chain(lhs.y.truncation.limbs.iter())
                    .chain(rhs.x.truncation.limbs.iter())
                    .chain(rhs.y.truncation.limbs.iter())
                    .map(|assigned| assigned.cell())
                    .collect();

                Ok((agg_instances, instances))
            },
        )
    }

    pub fn num_instance() -> Vec<usize> {
        // [..lhs, ..rhs]
        vec![4 * LIMBS]
    }

    pub fn accumulator_indices() -> Vec<(usize, usize)> {
        (0..4 * LIMBS).map(|idx| (0, idx)).collect()
    }
}

pub fn accumulator_native(snarks: &[&Snark]) -> (Vec<Fr>, Vec<u8>) {
    let accumulators = snarks
        .iter()
        .flat_map(|snark| {
            let mut transcript = PoseidonTranscript::<NativeLoader, _>::new(snark.proof.as_slice());
            let proof = Plonk::read_proof(
                &snark.svk,
                &snark.protocol,
                &snark.instances,
                &mut transcript,
            );
            Plonk::succinct_verify(&snark.svk, &snark.protocol, &snark.instances, &proof)
        })
        .collect_vec();

    let (accumulator, as_proof) = {
        let mut transcript = PoseidonTranscript::<NativeLoader, _>::new(Vec::new());
        let accumulator =
            As::create_proof(&Default::default(), &accumulators, &mut transcript, OsRng).unwrap();
        (accumulator, transcript.finalize())
    };
    let KzgAccumulator { lhs, rhs } = accumulator;
    let instances = [lhs.x, lhs.y, rhs.x, rhs.y]
        .map(fe_to_limbs::<_, _, LIMBS, BITS>)
        .concat();

    (instances, as_proof)
}

pub fn accumulator_ecc<'a>(
    loader: &Rc<Halo2Loader<'a>>,
    snarks: &[&Snark],
    as_proof: Value<&'_ [u8]>,
) -> (
    KzgAccumulator<G1Affine, Rc<Halo2Loader<'a>>>,
    Vec<Vec<SnarkInstanceColumnCells>>,
) {
    let assign_instances = |instances: &[Vec<Fr>]| {
        instances
            .iter()
            .map(|instances| {
                instances
                    .iter()
                    .map(|instance| loader.assign_scalar(Value::known(*instance)))
                    .collect_vec()
            })
            .collect_vec()
    };

    let mut all_instances = vec![];

    let accumulators = snarks
        .iter()
        .flat_map(|snark| {
            let protocol = snark.protocol.loaded(loader);
            let instances = assign_instances(&snark.instances);

            all_instances.push(
                instances
                    .iter()
                    .map(|f| f.iter().map(|f| f.clone().assigned().cell()).collect_vec())
                    .collect_vec(),
            );

            let mut transcript =
                PoseidonTranscript::<Rc<Halo2Loader>, _>::new(loader, snark.proof_value());
            let proof = Plonk::read_proof(&snark.svk, &protocol, &instances, &mut transcript);
            Plonk::succinct_verify(&snark.svk, &protocol, &instances, &proof)
        })
        .collect_vec();

    let acccumulator = {
        let mut transcript = PoseidonTranscript::<Rc<Halo2Loader>, _>::new(loader, as_proof);
        let proof = As::read_proof(&Default::default(), &accumulators, &mut transcript).unwrap();
        As::verify(&Default::default(), &accumulators, &proof).unwrap()
    };

    (acccumulator, all_instances)
}

#[cfg(test)]
mod tests {
    use halo2_base::halo2_proofs::{
        circuit::{Layouter, SimpleFloorPlanner, Value},
        dev::MockProver,
        plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
    };

    use crate::{
        chips::aggregation::snark::Snark,
        test::util::{advice_column_equality, get_snark, instance_column_equality},
        util::assign_private_input,
    };

    use super::*;

    #[derive(Clone, Debug)]
    pub struct BasicCircuitConfig {
        instance: Column<Instance>,
        advice: Column<Advice>,
    }

    #[derive(Clone, Default, Debug)]
    pub struct BasicCircuit {
        input: Fr,
    }

    impl Circuit<Fr> for BasicCircuit {
        type Config = BasicCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            BasicCircuitConfig {
                instance: instance_column_equality(meta),
                advice: advice_column_equality(meta),
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let input = assign_private_input(
                || "assign input",
                layouter.namespace(|| "assign input"),
                config.advice,
                Value::known(self.input),
            )?;

            layouter.constrain_instance(input.cell(), config.instance, 0)
        }
    }

    #[derive(Clone, Default, Debug)]
    pub struct BasicCircuit2 {
        input1: Fr,
        input2: Fr,
    }

    impl Circuit<Fr> for BasicCircuit2 {
        type Config = BasicCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            BasicCircuitConfig {
                instance: instance_column_equality(meta),
                advice: advice_column_equality(meta),
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let input1 = assign_private_input(
                || "assign input1",
                layouter.namespace(|| "assign input1"),
                config.advice,
                Value::known(self.input1),
            )?;

            let input2 = assign_private_input(
                || "assign input2",
                layouter.namespace(|| "assign input2"),
                config.advice,
                Value::known(self.input2),
            )?;

            layouter.constrain_instance(input1.cell(), config.instance, 0)?;
            layouter.constrain_instance(input2.cell(), config.instance, 1)
        }
    }

    #[derive(Clone)]
    pub struct AggregationCircuitConfig {
        aggregation_config: AggregationChipConfig,
        instance: Column<Instance>,
    }

    #[derive(Clone)]
    pub struct AggregationCircuit {
        snarks: Vec<Snark>,
        agg_instances: Vec<Fr>,
        as_proof: Vec<u8>,
    }

    impl AggregationCircuit {
        pub fn new(snarks: Vec<Snark>) -> Self {
            let snarks_ref: Vec<&Snark> = snarks.iter().collect();
            let (instances, as_proof) = accumulator_native(&snarks_ref);

            AggregationCircuit {
                snarks,
                agg_instances: instances,
                as_proof,
            }
        }

        pub fn instances(&self) -> Vec<Vec<Fr>> {
            // Instances to verify aggregation/recurssion part of the proof
            let mut instances = self.agg_instances.clone();

            // Verify original elements
            let snark_instances = self
                .snarks
                .iter()
                .flat_map(|s| s.instances.clone().into_iter().flatten().collect_vec())
                .collect_vec();

            instances.extend(snark_instances);

            vec![instances]
        }
    }

    impl Circuit<Fr> for AggregationCircuit {
        type Config = AggregationCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            self.clone()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            let params = AggregationChipConfigParams {
                strategy: halo2_ecc::fields::fp::FpStrategy::Simple,
                degree: 21,
                num_advice: 6,
                num_lookup_advice: 1,
                num_fixed: 1,
                lookup_bits: 20,
                limb_bits: 88,
                num_limbs: 3,
            };

            let instance = meta.instance_column();
            meta.enable_equality(instance);

            AggregationCircuitConfig {
                aggregation_config: AggregationChipConfig::configure(meta, params),
                instance,
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let agg_chip = AggregationChip::construct(config.aggregation_config);
            let snarks_ref: Vec<&Snark> = self.snarks.iter().collect();

            // Expose instances
            let (agg_instances, snark_instances) = agg_chip.aggregate(
                layouter.namespace(|| "aggregate"),
                &snarks_ref,
                Value::known(&self.as_proof),
            )?;

            // TODO: use less instances by following Scroll's strategy of keeping only last bit of y coordinate
            let mut layouter = layouter.namespace(|| "expose");
            for (i, cell) in agg_instances
                .into_iter()
                .chain(snark_instances.into_iter().flatten().flatten())
                .enumerate()
            {
                layouter.constrain_instance(cell, config.instance, i)?;
            }

            Ok(())
        }
    }

    pub fn gen_application_snark(i: usize) -> Snark {
        let circuit = BasicCircuit {
            input: Fr::from(i as u64),
        };
        get_snark(8, circuit, vec![Fr::from(i as u64)]).unwrap()
    }

    #[test]
    fn test_aggregation() {
        let mut snarks = [(); 3]
            .iter()
            .enumerate()
            .map(|(i, _)| gen_application_snark(i))
            .collect_vec();

        // Test with two different circuits with different inputs/instances
        let circuit = BasicCircuit2 {
            input1: Fr::from(10),
            input2: Fr::from(11),
        };
        let alt_snark = get_snark(8, circuit, vec![Fr::from(10), Fr::from(11)]).unwrap();
        snarks.push(alt_snark);

        let agg_circuit = AggregationCircuit::new(snarks);

        let prover = MockProver::<Fr>::run(21, &agg_circuit, agg_circuit.instances()).unwrap();
        prover.assert_satisfied();

        println!("Success!");
    }
}
