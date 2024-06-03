use crate::chips::is_less_than::IsLessThanChip;
use crate::chips::{
    binary_decomposition::BinaryDecompositionConfig, is_constant::IsConstantChip,
    merkle_path::merkle_root, poseidon::PoseidonConfig, swap::CondSwapChip,
};
use crate::data::{Insert, MerklePath, Note};
use crate::util::{assign_constant, assign_private_input};
use halo2_base::halo2_proofs::circuit::AssignedCell;
use halo2_base::halo2_proofs::{
    circuit::{Layouter, Value},
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, Error},
};
use zk_primitives::Element;

impl<const MERKLE_D: usize> Insert<MERKLE_D> {
    pub fn new(leaf: Element, path: MerklePath<MERKLE_D>) -> Self {
        Self { leaf, path }
    }

    pub fn padding_insert() -> Self {
        Insert::new(Note::padding_note().commitment(), MerklePath::default())
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
        less_than_chip: IsLessThanChip<Fr>,
    ) -> Result<InsertConstraintCells, Error> {
        // Witness new leaf
        let new_leaf = assign_private_input(
            || "new leaf witness",
            layouter.namespace(|| "new leaf witness"),
            advice,
            Value::known(self.leaf()),
        )?;

        // Witness null leaf
        let null_leaf = assign_constant(
            || "null leaf witness",
            layouter.namespace(|| "null leaf witness"),
            advice,
            Fr::zero(),
        )?;

        // Binary decomposition using RunningSum is a vec of AssignedCells containing the bits
        let decomposed_bits = layouter.assign_region(
            || "decompose",
            |mut region| {
                // We use non-struct because the merkle tree is not as big as the hash (i.e. we're only
                // interested in the last n bits)
                decompose.copy_decompose(&mut region, 0, new_leaf.clone(), 256, 256)
            },
        )?;

        // Zero
        let zero = assign_constant(
            || "assign zero bit",
            layouter.namespace(|| "zero bit"),
            advice,
            Fr::from(0),
        )?;

        // One
        let one: AssignedCell<Fr, Fr> = assign_constant(
            || "assign one bit",
            layouter.namespace(|| "one bit"),
            advice,
            Fr::from(1),
        )?;

        // Ensure insert hash is within modulus
        less_than_chip.assign(
            layouter.namespace(|| "less than modulus"),
            &Element::MODULUS
                .to_be_bits()
                .iter()
                .map(|b| if *b { one.clone() } else { zero.clone() })
                .collect::<Vec<_>>(),
            &decomposed_bits
                .clone()
                .into_iter()
                .rev()
                .collect::<Vec<_>>(),
        )?;

        // Witness all siblings
        let sibling_witnesses = self
            .path
            .siblings
            .iter()
            .map(|w| {
                assign_private_input(
                    || "leaf witness",
                    layouter.namespace(|| "leaf witness"),
                    advice,
                    Value::known(w.to_base()),
                )
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // Merge siblings with decomposed bits
        let siblings = sibling_witnesses
            .iter()
            .zip(decomposed_bits.iter().take(MERKLE_D - 1))
            .collect::<Vec<_>>();

        // Prove old root based on merkle path and null leaf
        let old_root = merkle_root(
            layouter.namespace(|| "old root"),
            swap_chip.clone(),
            poseidon_config.clone(),
            null_leaf,
            &siblings,
        )?;

        let new_root = merkle_root(
            layouter.namespace(|| "new root"),
            swap_chip,
            poseidon_config,
            new_leaf.clone(),
            &siblings,
        )?;

        // Padding check
        let is_padding =
            padding_constant_chip.assign(layouter.namespace(|| "is padding"), new_leaf.clone())?;

        Ok(InsertConstraintCells {
            new_leaf,
            old_root,
            new_root,
            is_padding,
        })
    }

    pub fn leaf(&self) -> Fr {
        self.leaf.into()
    }

    pub fn compute_null_root(&self) -> Fr {
        self.path.compute_null_root(self.leaf).into()
    }

    pub fn compute_leaf_root(&self) -> Fr {
        self.path.compute_root(self.leaf).into()
    }

    pub fn is_padding(&self) -> bool {
        self.leaf == Note::padding_note().commitment()
    }

    /// Public inputs to be used in proof
    ///  [new_leaf, old_root, new_root]
    pub fn public_inputs(&self) -> Vec<Fr> {
        let old_root = self.path.compute_null_root(self.leaf).into();
        let new_root = self.path.compute_root(self.leaf).into();

        vec![self.leaf(), old_root, new_root]
    }
}

#[derive(Debug)]
pub struct InsertConstraintCells {
    /// New leaf node witness
    pub new_leaf: AssignedCell<Fr, Fr>,
    /// Old root node witness
    pub old_root: AssignedCell<Fr, Fr>,
    /// New root node calculated from path and new leaf
    pub new_root: AssignedCell<Fr, Fr>,
    /// Is this padding?
    pub is_padding: AssignedCell<Fr, Fr>,
}

#[cfg(test)]
mod tests {
    use halo2_base::halo2_proofs::{
        circuit::SimpleFloorPlanner,
        dev::MockProver,
        plonk::{Circuit, ConstraintSystem, Error, Instance},
    };
    use zk_primitives::Element;

    use crate::{
        chips::{
            is_constant::IsConstantConfig, is_less_than::IsLessThanChipConfig, swap::CondSwapConfig,
        },
        constants::MERKLE_TREE_DEPTH,
        test::util::{advice_column_equality, instance_column_equality, poseidon_config},
    };

    use super::*;

    #[derive(Debug, Clone)]
    struct InsertCircuitConfig {
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        swap_config: CondSwapConfig,
        is_padding_config: IsConstantConfig<Fr>,
        binary_decomposition_config: BinaryDecompositionConfig<Fr, 1>,
        advice: Column<Advice>,
        instance: Column<Instance>,
        is_less_than: IsLessThanChipConfig,
    }

    #[derive(Default, Clone, Debug)]
    struct InsertCircuit {
        insert: Insert<MERKLE_TREE_DEPTH>,
    }

    impl Circuit<Fr> for InsertCircuit {
        type Config = InsertCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            let advices: [Column<Advice>; 5] = (0..5)
                .map(|_| advice_column_equality(meta))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            let q_range_check = meta.selector();

            InsertCircuitConfig {
                poseidon_config: poseidon_config(meta),
                swap_config: CondSwapChip::configure(meta, advices),
                // Padding chip
                is_padding_config: IsConstantChip::configure(
                    meta,
                    advices[0],
                    advices[1],
                    advices[2],
                    Note::padding_note().commitment().into(),
                ),
                binary_decomposition_config: BinaryDecompositionConfig::configure(
                    meta,
                    q_range_check,
                    advices[0],
                    advices[1],
                ),
                advice: advice_column_equality(meta),
                instance: instance_column_equality(meta),
                is_less_than: IsLessThanChip::configure(
                    meta,
                    [advices[0], advices[1], advices[2], advices[3]],
                ),
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let swap_chip = CondSwapChip::construct(config.swap_config);

            let insert_cells = self.insert.enforce_constraints(
                layouter.namespace(|| "insert"),
                config.advice,
                config.poseidon_config,
                config.binary_decomposition_config,
                swap_chip,
                IsConstantChip::construct(config.is_padding_config),
                IsLessThanChip::construct(config.is_less_than),
            )?;

            layouter.constrain_instance(insert_cells.new_leaf.cell(), config.instance, 0)?;
            layouter.constrain_instance(insert_cells.old_root.cell(), config.instance, 1)?;
            layouter.constrain_instance(insert_cells.new_root.cell(), config.instance, 2)?;

            Ok(())
        }
    }

    #[test]
    fn test_insert() {
        let k = 14;

        let leaf = Element::from(7u64); // random_fr();
        let path = MerklePath::default();
        let insert = Insert::new(leaf, path);

        let public_input = insert.public_inputs();
        let instance_columns = vec![public_input];
        let circuit = InsertCircuit { insert };

        let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }
}
