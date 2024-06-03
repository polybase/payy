use halo2_base::halo2_proofs::{
    circuit::{AssignedCell, Layouter},
    halo2curves::bn256::Fr,
    plonk::Error,
};
use poseidon_circuit::{
    poseidon::{
        primitives::{ConstantLength, Hash as PoseidonHash, P128Pow5T3},
        Hash,
    },
    Hashable,
};

pub use poseidon_circuit::poseidon::{Pow5Chip as PoseidonChip, Pow5Config as PoseidonConfig};

pub type P128Pow5T3Fr = P128Pow5T3<Fr>;

/// Hashes AssignedCells
///
/// **BUG** must be equal number of elements due to bug in posiedon impl
/// Fixed in Halo2 poseidon (https://github.com/zcash/halo2/pull/646), but not in Scroll poseidon.
///
pub fn poseidon_hash_gadget<const L: usize>(
    config: PoseidonConfig<Fr, 3, 2>,
    mut layouter: impl Layouter<Fr>,
    messages: [AssignedCell<Fr, Fr>; L],
) -> Result<AssignedCell<Fr, Fr>, Error> {
    let chip = PoseidonChip::construct(config);
    let hasher = Hash::<_, _, P128Pow5T3<Fr>, ConstantLength<L>, 3, 2>::init(
        chip,
        layouter.namespace(|| "init poseidon hasher"),
    )?;

    hasher.hash(layouter.namespace(|| "hash"), messages)
}

// TODO: make Element Hashable
pub fn poseidon_hash<F: Hashable, const L: usize>(message: [F; L]) -> F {
    PoseidonHash::<F, P128Pow5T3<F>, ConstantLength<L>, 3, 2>::init().hash(message)
}

#[cfg(test)]
mod tests {
    use crate::util::{assign_private_input, random_fr};

    use super::*;
    use halo2_base::halo2_proofs::{
        circuit::{SimpleFloorPlanner, Value},
        dev::MockProver,
        plonk::{Advice, Circuit, Column, ConstraintSystem, Instance},
    };
    use smirk::hash_merge;
    use snark_verifier::util::arithmetic::FieldExt;

    #[test]
    fn poseidon_hash_snapshot() {
        let result = poseidon_hash([Fr::from_u128(2), Fr::from_u128(3)]);

        // make sure the debug representation doesn't change so we can change the hash impl
        assert_eq!(
            format!("{result:?}"),
            "0x19014d18a3179c5731155fcb7b6da422f456bccbd6da9dbc7df0f8dc6d4938ed"
        );

        assert_eq!(
            result,
            Fr::from(hash_merge([2, 3].map(zk_primitives::Element::new))),
        )
    }

    #[derive(Debug, Clone)]
    struct PoseidonCircuitConfig {
        advices: [Column<Advice>; 4],
        instance: Column<Instance>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
    }

    #[derive(Debug, Default, Clone)]
    struct PoseidonCircuit {
        left: Fr,
        right: Fr,
    }

    impl Circuit<Fr> for PoseidonCircuit {
        type Config = PoseidonCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> PoseidonCircuitConfig {
            let instance = meta.instance_column();
            meta.enable_equality(instance);

            let advices = [
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

            let poseidon_config = PoseidonChip::configure::<P128Pow5T3<Fr>>(
                meta,
                advices[0..3].try_into().unwrap(),
                advices[3],
                lagrange_coeffs[2..5].try_into().unwrap(),
                lagrange_coeffs[5..8].try_into().unwrap(),
            );

            PoseidonCircuitConfig {
                advices,
                instance,
                poseidon_config,
            }
        }

        fn synthesize(
            &self,
            config: PoseidonCircuitConfig,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            // Witness left
            let left = assign_private_input(
                || "assign left",
                layouter.namespace(|| "assign left"),
                config.advices[0],
                Value::known(self.left),
            )?;

            // Witness right
            let right = assign_private_input(
                || "assign right",
                layouter.namespace(|| "assign right"),
                config.advices[0],
                Value::known(self.right),
            )?;

            let combined = poseidon_hash_gadget(
                config.poseidon_config,
                layouter.namespace(|| "combine"),
                [left, right],
            )?;

            layouter.constrain_instance(combined.cell(), config.instance, 0)?;

            Ok(())
        }
    }

    #[test]
    fn test_poseidon() {
        let k = 7;
        let left = random_fr();
        let right = random_fr();
        let combined = poseidon_hash([left, right]);

        let circuit = PoseidonCircuit { left, right };

        let prover = MockProver::<Fr>::run(k, &circuit, vec![vec![combined]]).unwrap();
        prover.assert_satisfied();
    }
}
