use crate::{
    chips::{
        binary_decomposition::BinaryDecompositionConfig,
        is_constant::{IsConstantChip, IsConstantConfig},
        is_less_than::{IsLessThanChip, IsLessThanChipConfig},
        poseidon::{P128Pow5T3Fr, PoseidonChip, PoseidonConfig},
        swap::{CondSwapChip, CondSwapConfig},
    },
    data::{Batch, Note},
};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    halo2curves::bn256::Fr,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};

#[derive(Clone, Debug)]
pub struct BatchCircuitConfig {
    instance: Column<Instance>,
    advices: [Column<Advice>; 5],
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
    binary_decomposition_config: BinaryDecompositionConfig<Fr, 1>,
    swap_config: CondSwapConfig,
    is_padding_config: IsConstantConfig<Fr>,
    is_less_than: IsLessThanChipConfig,
}

impl<const N: usize, const M: usize> Circuit<Fr> for Batch<N, M> {
    type FloorPlanner = SimpleFloorPlanner;
    type Config = BatchCircuitConfig;

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

        BatchCircuitConfig {
            advices,
            instance,
            poseidon_config,
            binary_decomposition_config,
            swap_config,
            is_padding_config,
            is_less_than,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        // Get the public instances
        let cells = self.enforce_constraints(
            layouter.namespace(|| "enforce insert constraints"),
            config.advices[0],
            config.poseidon_config,
            config.binary_decomposition_config,
            CondSwapChip::construct(config.swap_config),
            IsConstantChip::construct(config.is_padding_config),
            &IsLessThanChip::construct(config.is_less_than),
        )?;

        // Constrain verify aggregation cells to public inputs
        self.enforce_instances(
            layouter.namespace(|| "enforce insert instances"),
            config.instance,
            cells,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::MERKLE_TREE_DEPTH,
        data::{Insert, MerklePath},
        test::rollup::Rollup,
    };
    use halo2_base::halo2_proofs::dev::MockProver;
    use zk_primitives::Element;

    use super::*;

    #[test]
    fn test_batch_insert_pad_insert_pad() {
        let k = 16;

        let mut rollup = Rollup::new();
        let old_root = rollup.root_hash();

        let padding_leaf = Note::padding_note().commitment();
        let padding_path = MerklePath::default();
        let padding_insert = Insert::new(padding_leaf, padding_path);

        let leaf_1 = Element::from(3u64);
        let leaf_1_path = rollup.merkle_path(leaf_1);
        let insert_1 = Insert::new(leaf_1, leaf_1_path);

        // Update the tree with the new insert, so leaf_2 path is valid
        rollup.tree.insert(leaf_1, ()).unwrap();

        let leaf_2 = Element::from(2u64);
        let leaf_2_path = rollup.merkle_path(leaf_2);
        let insert_2 = Insert::new(leaf_2, leaf_2_path);

        rollup.tree.insert(leaf_2, ()).unwrap();

        let circuit = Batch::<4, MERKLE_TREE_DEPTH>::new([
            insert_1,
            padding_insert.clone(),
            insert_2,
            padding_insert,
        ]);

        let instances = circuit.public_inputs();
        let new_root = rollup.root_hash();

        assert_eq!(instances.len(), 6);
        assert_eq!(instances[0], old_root.to_base());
        assert_eq!(instances[1], new_root.to_base());
        assert_eq!(instances[2], leaf_1.to_base());
        assert_eq!(instances[3], padding_leaf.to_base());
        assert_eq!(instances[4], leaf_2.to_base());
        assert_eq!(instances[5], padding_leaf.to_base());

        let prover = MockProver::<Fr>::run(k, &circuit, vec![instances]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_batch_pad_insert_pad_insert() {
        let k = 16;

        let mut rollup = Rollup::new();
        let old_root = rollup.root_hash();

        let padding_leaf = Note::padding_note().commitment();
        let padding_path = MerklePath::default();
        let padding_insert = Insert::new(padding_leaf, padding_path);

        let leaf_1 = Element::from(3u64);
        let leaf_1_path = rollup.merkle_path(leaf_1);
        let insert_1 = Insert::new(leaf_1, leaf_1_path);

        // Update the tree with the new insert, so leaf_2 path is valid
        rollup.tree.insert(leaf_1, ()).unwrap();

        let leaf_2 = Element::from(2u64);
        let leaf_2_path = rollup.merkle_path(leaf_2);
        let insert_2 = Insert::new(leaf_2, leaf_2_path);

        rollup.tree.insert(leaf_2, ()).unwrap();

        let circuit = Batch::<4, MERKLE_TREE_DEPTH>::new([
            padding_insert.clone(),
            insert_1,
            padding_insert,
            insert_2,
        ]);

        let instances = circuit.public_inputs();
        let new_root = rollup.root_hash();

        assert_eq!(instances.len(), 6);
        assert_eq!(instances[0], old_root.to_base());
        assert_eq!(instances[1], new_root.to_base());
        assert_eq!(instances[2], padding_leaf.to_base());
        assert_eq!(instances[3], leaf_1.to_base());
        assert_eq!(instances[4], padding_leaf.to_base());
        assert_eq!(instances[5], leaf_2.to_base());

        let prover = MockProver::<Fr>::run(k, &circuit, vec![circuit.public_inputs()]).unwrap();
        prover.assert_satisfied();
    }
}
