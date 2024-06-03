use crate::{
    chips::{
        aggregation::{
            aggregate::{accumulator_native, AggregationChip},
            snark::Snark,
        },
        binary_decomposition::BinaryDecompositionConfig,
        is_constant::IsConstantChip,
        is_less_than::IsLessThanChip,
        poseidon::PoseidonConfig,
        swap::CondSwapChip,
    },
    data::{Batch as BatchInsert, ParameterSet, Utxo},
    params::load_params,
    util::keygen_from_params,
    CircuitKind,
};
use halo2_base::halo2_proofs::{
    circuit::{Cell, Layouter, Value},
    halo2curves::bn256::{Fr, G1Affine},
    plonk::{Advice, Column, Error, Instance, ProvingKey, VerifyingKey},
};
use itertools::Itertools;

#[derive(Clone, Debug)]
pub struct AggregateUtxo<const UTXO_N: usize, const MERKLE_D: usize, const LEAVES: usize> {
    /// UTXO to aggregate
    utxo: [Snark; UTXO_N],

    /// Insert for the UTXO
    insert: BatchInsert<LEAVES, MERKLE_D>,

    /// Instances used to verify the proof
    pub agg_instances: Vec<Fr>,

    /// Private witness to proof
    proof: Vec<u8>,
}

impl<const UTXO_N: usize, const MERKLE_D: usize, const LEAVES: usize>
    AggregateUtxo<UTXO_N, MERKLE_D, LEAVES>
{
    pub fn new(utxo: [Snark; UTXO_N], insert: BatchInsert<LEAVES, MERKLE_D>) -> Self {
        let snarks = Self::snarks(&utxo);

        let (agg_instances, proof) = accumulator_native(&snarks);

        Self {
            // previous_agg,
            utxo,
            insert,
            agg_instances,
            proof,
        }
    }

    fn snarks(utxo: &[Snark; UTXO_N]) -> Vec<&Snark> {
        utxo.iter().collect_vec()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        instance: Column<Instance>,
        advice: Column<Advice>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        decompose: BinaryDecompositionConfig<Fr, 1>,
        swap_chip: CondSwapChip<Fr>,
        padding_constant_chip: IsConstantChip<Fr>,
        aggregation_chip: &AggregationChip,
        is_less_than_chip: &IsLessThanChip<Fr>,
    ) -> Result<(), Error> {
        // Gather snarks
        let snarks = Self::snarks(&self.utxo);

        // Aggregate proofs
        let (agg_cells, utxo_snarks) = aggregation_chip.aggregate(
            layouter.namespace(|| "aggregate"),
            &snarks,
            Value::known(&self.proof),
        )?;

        // // Prove merkle insert logic
        let insert = self.insert.enforce_constraints(
            layouter.namespace(|| "insert"),
            advice,
            poseidon_config,
            decompose,
            swap_chip,
            padding_constant_chip,
            is_less_than_chip,
        )?;

        // Constrain verify aggregation cells to public inputs
        for (i, cell) in agg_cells.iter().enumerate() {
            layouter.constrain_instance(*cell, instance, i)?;
        }
        let mut instance_counter = agg_cells.len();

        // Constrain old root
        layouter.constrain_instance(insert.old_root.cell(), instance, instance_counter)?;
        instance_counter += 1;

        // Constrain new root
        layouter.constrain_instance(insert.new_root.cell(), instance, instance_counter)?;
        instance_counter += 1;

        // Constrain recent roots & get leafs
        let mut utxo_leafs: Vec<&Cell> = vec![];
        for (i, snark_instances) in utxo_snarks.iter().enumerate() {
            let instance_counter_base = instance_counter + (i * 3);

            // We only use one instance column per snark, so no need to use the others
            let snark_instances = snark_instances.first().unwrap();

            // First instance is the root (pass through from input proof to agg proof)
            let utxo_root = snark_instances.first().expect("Missing utxo root instance");
            layouter.constrain_instance(*utxo_root, instance, instance_counter_base)?;

            // Second instance is kind (mint, burn, transfer)
            let hash: &Cell = snark_instances
                .get(1)
                .expect("Missing mint/burn hash instance");
            layouter.constrain_instance(*hash, instance, instance_counter_base + 1)?;

            // Third instance is value (constrain to output instance)
            let value = snark_instances
                .get(2)
                .expect("Missing mint/burn value instance");
            layouter.constrain_instance(*value, instance, instance_counter_base + 2)?;

            // Remaining instances are leaves
            utxo_leafs.extend(snark_instances.iter().skip(3));
        }

        // Constrain utxo and insert leafs to be equal, only for the amount of utxo leafs that should exist. Remaining
        // insert leafs are for mints.
        for (utxo_leaf, insert_leaf) in utxo_leafs.iter().zip(insert.leafs.iter()).take(LEAVES) {
            layouter.assign_region(
                || "leaf equality",
                |mut region| {
                    region.constrain_equal(**utxo_leaf, insert_leaf.cell())?;
                    Ok(())
                },
            )?;
        }

        Ok(())
    }

    // TODO: we should use a typed system for extracting cells from the snark instances
    pub fn public_inputs(&self) -> Vec<Fr> {
        let mut instances = vec![];

        // Add verify instances (12)
        instances.extend(self.agg_instances.clone());

        // Add old root (1)
        instances.push(self.old_root());

        // Add new root (1)
        instances.push(self.new_root());

        // Recent root (1), mint/burn hash (1), mint/burn value (1) (= 3 per UTXO)
        instances.extend(self.utxo_public_inputs());

        instances
    }

    pub fn utxo_public_inputs(&self) -> Vec<&Fr> {
        self.utxo
            .iter()
            .flat_map(|snark| {
                let recent_root = &snark.instances[0][0];
                let mb_hash = &snark.instances[0][1];
                let value = &snark.instances[0][2];

                vec![recent_root, mb_hash, value]
            })
            .collect_vec()
    }

    pub fn recent_roots(&self) -> Vec<&Fr> {
        self.utxo
            .iter()
            .map(|snark| &snark.instances[0][0])
            .collect_vec()
    }

    pub fn leafs(&self) -> Vec<Fr> {
        self.insert.leafs()
    }

    pub fn old_root(&self) -> Fr {
        self.insert.old_root()
    }

    pub fn new_root(&self) -> Fr {
        self.insert.new_root()
    }

    pub fn snark(&self, params: ParameterSet) -> Result<Snark, crate::Error> {
        let pk = CircuitKind::AggUtxo.pk();
        Snark::create(
            Self::default(),
            vec![self.public_inputs()],
            load_params(params),
            pk,
        )
        .map_err(crate::Error::err)
    }

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }
}

impl<const UTXO_N: usize, const MERKLE_D: usize, const LEAVES: usize> Default
    for AggregateUtxo<UTXO_N, MERKLE_D, LEAVES>
{
    fn default() -> Self {
        let utxo = (0..UTXO_N)
            .map(|_| Utxo::<MERKLE_D>::default().snark(CircuitKind::Utxo))
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .try_into()
            .unwrap();

        let insert = BatchInsert::default();

        Self::new(utxo, insert)
    }
}
