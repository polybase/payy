use crate::{
    chips::{
        aggregation::snark::Snark, binary_decomposition::BinaryDecompositionConfig,
        is_constant::IsConstantChip, is_less_than::IsLessThanChip, poseidon::PoseidonConfig,
        swap::CondSwapChip,
    },
    data::{Batch, Insert, ParameterSet},
    params::load_params,
    proof::Proof,
    util::{assign_private_input, keygen_from_params},
};
use halo2_base::halo2_proofs::{
    circuit::{AssignedCell, Layouter, Value},
    halo2curves::bn256::{Bn256, Fr, G1Affine},
    plonk::{Advice, Column, Error, Instance, ProvingKey, VerifyingKey},
    poly::kzg::commitment::ParamsKZG,
};
use rand::RngCore;

use super::InsertConstraintCells;

impl<const INSERTS: usize, const MERKLE_D: usize> Batch<INSERTS, MERKLE_D> {
    pub fn new(inserts: [Insert<MERKLE_D>; INSERTS]) -> Self {
        Self { inserts }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        advice: Column<Advice>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        decompose: BinaryDecompositionConfig<Fr, 1>,
        swap_chip: CondSwapChip<Fr>,
        padding_constant_chip: IsConstantChip<Fr>,
        is_less_than_chip: &IsLessThanChip<Fr>,
    ) -> Result<BatchConstraintCells, Error> {
        let mut leafs = vec![];

        // Witness old root
        let old_root: AssignedCell<Fr, Fr> = assign_private_input(
            || "old root witness",
            layouter.namespace(|| "old root witness"),
            advice,
            Value::known(self.old_root()),
        )?;

        // Old root
        let mut last_new_root = old_root.clone();

        for insert in self.inserts.iter() {
            let InsertConstraintCells {
                old_root,
                new_root,
                new_leaf,
                is_padding,
            } = insert.enforce_constraints(
                layouter.namespace(|| "insert"),
                advice,
                poseidon_config.clone(),
                decompose,
                swap_chip.clone(),
                padding_constant_chip.clone(),
                is_less_than_chip.clone(),
            )?;

            // Store leafs for instance constraint
            leafs.push(new_leaf);

            // TODO(perf) should we remove this, as this adds extra ZK work and we could calculate
            // this in the merkle tree
            let (old_root, _) = swap_chip.swap_assigned(
                layouter.namespace(|| "swap padding"),
                (&old_root, &last_new_root),
                &is_padding,
            )?;

            let (new_root, _) = swap_chip.swap_assigned(
                layouter.namespace(|| "swap padding"),
                (&new_root, &old_root),
                &is_padding,
            )?;

            // Check that the old root is the same as the last new root
            layouter.assign_region(
                || "constrain roots",
                |mut region| region.constrain_equal(last_new_root.cell(), old_root.cell()),
            )?;

            last_new_root = new_root;
        }

        Ok(BatchConstraintCells {
            old_root,
            new_root: last_new_root,
            leafs,
        })
    }

    pub fn enforce_instances(
        &self,
        mut layouter: impl Layouter<Fr>,
        instance: Column<Instance>,
        cells: BatchConstraintCells,
    ) -> Result<(), Error> {
        let BatchConstraintCells {
            old_root,
            new_root,
            leafs,
        } = cells;

        // Old root
        layouter.constrain_instance(old_root.cell(), instance, 0)?;

        // New root
        layouter.constrain_instance(new_root.cell(), instance, 1)?;

        // Check constraints leafs
        for (i, leaf) in leafs.iter().enumerate() {
            // Constrain leaf to be the same as the leaf in the instance, +2 as the first
            // two contraints are old_root and new_root
            layouter.constrain_instance(leaf.cell(), instance, i + 2)?;
        }

        Ok(())
    }

    pub fn old_root(&self) -> Fr {
        // Calculate the old root from first entry
        self.inserts
            .first()
            .expect("at least one insert")
            .compute_null_root()
    }

    pub fn new_root(&self) -> Fr {
        self.inserts
            .iter()
            .rev()
            .find(|i| !i.is_padding())
            .map(|i| i.compute_leaf_root())
            .unwrap_or(self.old_root())
    }

    pub fn leafs(&self) -> Vec<Fr> {
        self.inserts
            .iter()
            .map(|insert| insert.leaf.to_base())
            .collect::<Vec<_>>()
    }

    /// Public instances needed to construct proof
    pub fn public_inputs(&self) -> Vec<Fr> {
        // Calculate the old root from first entry
        let old_root = self.old_root();

        // Calculate new root from last entry
        let new_root = self.new_root();

        // Collect all leafs
        let leafs = self.leafs();

        vec![old_root, new_root].into_iter().chain(leafs).collect()
    }

    pub fn snark(&self, params: ParameterSet) -> Result<Snark, Error> {
        let (pk, _) = self.keygen(params);

        Snark::create(
            Self::default(),
            vec![self.public_inputs()],
            load_params(params),
            &pk,
        )
    }

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }

    pub fn prove(
        &self,
        params: &ParamsKZG<Bn256>,
        pk: &ProvingKey<G1Affine>,
        rng: impl RngCore,
    ) -> Result<Proof, Error> {
        let circuit = Self::default();
        let instance = self.public_inputs();
        let instances = &[instance.as_slice()];
        Proof::create(params, pk, circuit, instances, rng)
    }
}

pub struct BatchConstraintCells {
    pub old_root: AssignedCell<Fr, Fr>,
    pub new_root: AssignedCell<Fr, Fr>,
    pub leafs: Vec<AssignedCell<Fr, Fr>>,
}
