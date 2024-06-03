use super::note::NoteConstraintCells;
use crate::{
    chips::{
        is_constant::IsConstantChip,
        merkle_path::MerklePathInclusionConstrainCells,
        poseidon::{poseidon_hash_gadget, PoseidonConfig},
        swap::CondSwapChip,
    },
    data::{InputNote, MerklePath, Note},
    util::{assign_constant, assign_private_input},
};
use halo2_base::halo2_proofs::{
    circuit::{AssignedCell, Layouter, Value},
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, Error},
};
use zk_primitives::Element;

impl<const MERKLE_D: usize> InputNote<MERKLE_D> {
    pub fn new(note: Note, secret_key: Element, merkle_path: MerklePath<MERKLE_D>) -> Self {
        InputNote {
            note,
            secret_key,
            merkle_path,
        }
    }

    /// Deterministic padding note
    pub fn padding_note() -> Self {
        InputNote {
            note: Note::padding_note(),
            secret_key: Element::ZERO,
            merkle_path: MerklePath::default(),
        }
    }

    pub fn output_note(&self, address: Element, value: Element) -> Note {
        Note::new_with_source(address, value, self.note.address)
    }

    pub fn is_padding(&self) -> bool {
        self.note.commitment() == Note::padding_note().commitment()
    }

    /// Get the nullifier for an input note
    pub fn nullifer(&self) -> Element {
        self.note.nullifier(self.secret_key)
    }

    /// Enforces constraints for the input note (includes default note constraints, plus additional
    /// constraints to prove spending of note is allowable)
    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        advice: Column<Advice>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        swap_chip: CondSwapChip<Fr>,
        is_zero_chip: IsConstantChip<Fr>,
    ) -> Result<InputNoteConstraintCells, Error> {
        // First we need to check the std note constraints
        let note_commitment_cells = self.note.enforce_constraints(
            layouter.namespace(|| "input note enforce commitment"),
            advice,
            poseidon_config.clone(),
            is_zero_chip,
            swap_chip.clone(),
        )?;

        // Check input note commitment is in an existing merkle root
        let MerklePathInclusionConstrainCells { root } =
            self.merkle_path.enforce_inclusion_constraints(
                layouter.namespace(|| "leaf in tree"),
                self.note.commitment().into(),
                note_commitment_cells.cm.clone(),
                poseidon_config.clone(),
                swap_chip,
            )?;

        // Witness secret_key
        let secret_key = assign_private_input(
            || "secret key witness",
            layouter.namespace(|| "secret key witness"),
            advice,
            Value::known(self.secret_key().into()),
        )?;

        let padding = assign_constant(
            || "padding witness",
            layouter.namespace(|| "padding witness"),
            advice,
            Fr::zero(),
        )?;

        // Verify that the address matches the secret key
        let verified_address = poseidon_hash_gadget(
            poseidon_config.clone(),
            layouter.namespace(|| "verify address"),
            [secret_key.clone(), padding.clone()],
        )?;

        // Constrain address to be the same as verified address
        // TODO: are we allowed to constrain_equal between two different regions cells?
        layouter.assign_region(
            || "constrain address",
            |mut region| {
                region.constrain_equal(
                    verified_address.cell(),
                    note_commitment_cells.address.cell(),
                )
            },
        )?;

        // Generate the nullifier
        let nullifier = poseidon_hash_gadget(
            poseidon_config,
            layouter.namespace(|| "nullifer hash"),
            [
                note_commitment_cells.cm.clone(),
                secret_key.clone(),
                note_commitment_cells.psi.clone(),
                padding.clone(),
            ],
        )?;

        Ok(InputNoteConstraintCells {
            commitment: note_commitment_cells,
            nullifier,
            root,
            secret_key,
            zero: padding,
        })
    }

    pub fn secret_key(&self) -> Element {
        self.secret_key
    }

    pub fn recent_root(&self) -> Element {
        self.merkle_path.compute_root(self.note.commitment())
    }

    pub fn note(&self) -> &Note {
        &self.note
    }

    pub fn value(&self) -> Element {
        self.note.value()
    }

    pub fn source(&self) -> Element {
        self.note.source()
    }

    pub fn commitment(&self) -> Element {
        self.note.commitment()
    }
}

pub struct InputNoteConstraintCells {
    /// Note commitment constaint cells
    pub commitment: NoteConstraintCells,
    /// Nullifier to be inserted into hte tree
    pub nullifier: AssignedCell<Fr, Fr>,
    /// recent root commitment that merkle tree path was verified against
    pub root: AssignedCell<Fr, Fr>,
    /// Secret key for the address, required to spend a note
    pub secret_key: AssignedCell<Fr, Fr>,
    /// Padding
    pub zero: AssignedCell<Fr, Fr>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chips::{is_constant::IsConstantConfig, swap::CondSwapConfig},
        constants::MERKLE_TREE_DEPTH,
        test::util::{
            advice_column_equality, instance_column_equality, is_padding_config, poseidon_config,
        },
    };
    use halo2_base::halo2_proofs::{
        circuit::{Layouter, SimpleFloorPlanner},
        dev::MockProver,
        plonk::{Advice, Circuit, Column, Error, Instance},
    };
    use rand::thread_rng;
    use smirk::hash_merge;

    #[derive(Clone, Debug)]
    struct InputNoteCircuitConfig {
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        swap_config: CondSwapConfig,
        advice: Column<Advice>,
        instance: Column<Instance>,
        is_zero_config: IsConstantConfig<Fr>,
    }

    #[derive(Default, Debug, Clone)]
    struct InputNoteCircuit {
        input_note: InputNote<MERKLE_TREE_DEPTH>,
    }

    impl Circuit<Fr> for InputNoteCircuit {
        type Config = InputNoteCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(
            meta: &mut halo2_base::halo2_proofs::plonk::ConstraintSystem<Fr>,
        ) -> Self::Config {
            let advices: [Column<Advice>; 5] = (0..5)
                .map(|_| advice_column_equality(meta))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            InputNoteCircuitConfig {
                poseidon_config: poseidon_config(meta),
                swap_config: CondSwapChip::configure(meta, advices),
                is_zero_config: is_padding_config(meta, Fr::zero()),
                advice: advice_column_equality(meta),
                instance: instance_column_equality(meta),
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let input_note_cells = self.input_note.enforce_constraints(
                layouter.namespace(|| "input note"),
                config.advice,
                config.poseidon_config,
                CondSwapChip::construct(config.swap_config),
                IsConstantChip::construct(config.is_zero_config),
            )?;

            layouter.constrain_instance(
                input_note_cells.commitment.cm.cell(),
                config.instance,
                0,
            )?;
            layouter.constrain_instance(input_note_cells.nullifier.cell(), config.instance, 1)?;
            layouter.constrain_instance(input_note_cells.root.cell(), config.instance, 2)?;

            Ok(())
        }
    }

    #[test]
    fn test_input_note() {
        let k = 14;
        let pk = Element::secure_random(thread_rng());
        let address = hash_merge([pk, Element::ZERO]);

        let note = Note::new(address, Element::from(100u64));
        let path = MerklePath::default();
        let input_note = InputNote::new(note.clone(), pk, path.clone());

        let nullifier = input_note.nullifer();
        let root = path.compute_root(note.commitment());

        let public_input = vec![note.commitment().into(), nullifier.into(), root.into()];
        let instance_columns = vec![public_input];
        let circuit = InputNoteCircuit { input_note };

        let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }
}
