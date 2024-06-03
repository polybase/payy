#![allow(clippy::assign_op_pattern)]

use crate::{
    chips::{
        is_constant::IsConstantChip,
        poseidon::{poseidon_hash, poseidon_hash_gadget, PoseidonConfig},
        swap::CondSwapChip,
    },
    constants::NOTE_RCM_EXT,
    data::Note,
    util::{assign_constant, assign_private_input, random_fr},
};
use halo2_base::halo2_proofs::{
    circuit::{AssignedCell, Layouter, Value},
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, Error},
};
use smirk::hash_merge;
use zk_primitives::Element;

impl Note {
    pub fn new(address: Element, value: Element) -> Self {
        Self::new_with_source(address, value, address)
    }

    pub(crate) fn new_with_source(address: Element, value: Element, source: Element) -> Self {
        let rseed = random_fr();
        let psi = poseidon_hash([rseed, Fr::from(NOTE_RCM_EXT as u64)]);

        Self::restore(address, psi.into(), value, source)
    }

    pub fn restore(address: Element, psi: Element, value: Element, source: Element) -> Self {
        Note {
            address,
            psi,
            value,
            source,
            token: "USDC".to_string(),
        }
    }

    /// Deterministic padding note
    pub fn padding_note() -> Self {
        let zero_hash: Element = poseidon_hash([Fr::zero(), Fr::zero()]).into();
        Note {
            address: zero_hash,
            psi: Element::ZERO,
            value: Element::ZERO,
            source: zero_hash,
            token: "USDC".to_string(),
        }
    }

    /// Hash/commitment for the note
    pub fn commitment(&self) -> Element {
        if self.value() == Element::ZERO {
            return Element::ZERO;
        }

        hash_merge([
            self.value(),
            self.address,
            self.psi,
            self.source,
            // TODO: should these be zero?
            Element::ONE,
            Element::ONE,
        ])
    }

    pub fn is_padding(&self) -> bool {
        self.commitment() == Note::padding_note().commitment() || self.commitment() == Element::ZERO
    }

    pub fn nullifier(&self, secret_key: Element) -> Element {
        if self.is_padding() {
            Note::padding_note().commitment()
        } else {
            hash_merge([self.commitment(), secret_key, self.psi(), Element::ZERO])
        }
    }

    /// Enforces constraints for the note
    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        advice: Column<Advice>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        is_zero_chip: IsConstantChip<Fr>,
        swap_chip: CondSwapChip<Fr>,
    ) -> Result<NoteConstraintCells, Error> {
        // Reconstruct the commitment using its parts by witnessing each of the parts/values
        // and then generating the commitment using those witnessed values. Those witnessed values
        // can later be used knowing they came from the commitment
        // [value, address, psi]

        // Witness zero
        let zero = assign_constant(
            || "zero witness",
            layouter.namespace(|| "zero witness"),
            advice,
            Fr::zero(),
        )?;

        // Witness value
        let value = assign_private_input(
            || "value witness",
            layouter.namespace(|| "value witness"),
            advice,
            Value::known(self.value().into()),
        )?;

        // Witness address
        let address = assign_private_input(
            || "address witness",
            layouter.namespace(|| "address witness"),
            advice,
            Value::known(self.address().into()),
        )?;

        // Witness psi
        let psi = assign_private_input(
            || "psi witness",
            layouter.namespace(|| "psi witness"),
            advice,
            Value::known(self.psi().into()),
        )?;

        // Witness Source
        let source: AssignedCell<Fr, Fr> = assign_private_input(
            || "source witness",
            layouter.namespace(|| "source witness"),
            advice,
            Value::known(self.source().into()),
        )?;

        // Witness Version
        let version: AssignedCell<Fr, Fr> = assign_private_input(
            || "version witness",
            layouter.namespace(|| "version witness"),
            advice,
            Value::known(Fr::one()),
        )?;

        // Calculate the incoming commitment, must be equal number of elements!
        let cm = poseidon_hash_gadget(
            poseidon_config,
            layouter.namespace(|| "note commitment hash"),
            [
                value.clone(),
                address.clone(),
                psi.clone(),
                source.clone(),
                version.clone(),
                version,
            ],
        )?;

        // Padding check
        let is_value_zero =
            is_zero_chip.assign(layouter.namespace(|| "is zero/padding"), value.clone())?;

        // Swap cm to zero if padding
        let (cm, _) = swap_chip.swap_assigned(
            layouter.namespace(|| "swap cm if padding note"),
            (&cm, &zero),
            &is_value_zero,
        )?;

        Ok(NoteConstraintCells {
            value,
            address,
            cm,
            is_padding: is_value_zero,
            source,
            psi,
        })
    }

    pub fn value(&self) -> Element {
        self.value
    }

    pub fn address(&self) -> Element {
        self.address
    }

    pub fn psi(&self) -> Element {
        self.psi
    }

    pub fn source(&self) -> Element {
        self.source
    }
}

pub struct NoteConstraintCells {
    /// AssignedCell holding the notes value
    pub value: AssignedCell<Fr, Fr>,
    /// AssignedCell holding the notes address
    pub address: AssignedCell<Fr, Fr>,
    /// AssignedCell holding the notes commitment hash
    pub cm: AssignedCell<Fr, Fr>,
    // AssignedCell identifying if this note is for padding only
    pub is_padding: AssignedCell<Fr, Fr>,
    /// AssignedCell for the source of note
    pub source: AssignedCell<Fr, Fr>,
    /// PSI for the source of note
    pub psi: AssignedCell<Fr, Fr>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chips::{
            is_constant::{IsConstantChip, IsConstantConfig},
            swap::CondSwapConfig,
        },
        test::util::{
            advice_column_equality, instance_column_equality, is_padding_config, poseidon_config,
            swap_config,
        },
    };
    use halo2_base::halo2_proofs::{
        arithmetic::Field,
        circuit::SimpleFloorPlanner,
        dev::MockProver,
        plonk::{Circuit, ConstraintSystem, Instance},
    };
    use rand::rngs::OsRng as rng;

    #[test]
    fn test_serde_note() {
        let note = Note {
            address: Element::random(rng).get_insecure(),
            psi: Element::random(rng).get_insecure(),
            value: Element::from(100u64),
            source: Element::random(rng).get_insecure(),
            token: "USDC".to_string(),
        };

        // Serialize note
        let note_json = serde_json::to_string(&note).unwrap();

        // Deserialize note
        let deserialized_note: Note = serde_json::from_str(&note_json).unwrap();

        assert_eq!(note, deserialized_note);
    }

    #[derive(Clone, Debug, Default)]
    struct NoteCircuit {
        note: Note,
    }

    #[derive(Clone, Debug)]
    struct NoteCircuitConfig {
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        advice: Column<Advice>,
        instance: Column<Instance>,
        swap_config: CondSwapConfig,
        is_zero_config: IsConstantConfig<Fr>,
    }

    impl NoteCircuit {
        pub fn new(note: Note) -> Self {
            NoteCircuit { note }
        }
    }

    impl Circuit<Fr> for NoteCircuit {
        type Config = NoteCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            NoteCircuitConfig {
                poseidon_config: poseidon_config(meta),
                is_zero_config: is_padding_config(meta, Fr::zero()),
                swap_config: swap_config(meta),
                advice: advice_column_equality(meta),
                instance: instance_column_equality(meta),
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let is_zero_chip = IsConstantChip::construct(config.is_zero_config);

            let note_cells = self.note.enforce_constraints(
                layouter.namespace(|| "note constraints"),
                config.advice,
                config.poseidon_config,
                is_zero_chip,
                CondSwapChip::construct(config.swap_config),
            )?;

            layouter.constrain_instance(note_cells.cm.cell(), config.instance, 0)?;
            layouter.constrain_instance(note_cells.is_padding.cell(), config.instance, 1)?;

            Ok(())
        }
    }

    #[test]
    fn test_note() {
        let k = 8;
        let address = Fr::random(rng);

        let note = Note::new(address.into(), Element::from(100u64));
        let cm = note.commitment();

        let public_input = vec![cm.into(), Fr::zero()];
        let instance_columns = vec![public_input];
        let circuit = NoteCircuit::new(note);

        let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_padding() {
        let k = 8;
        let note = Note::padding_note();
        let cm = note.commitment();

        let public_input = vec![cm.into(), Fr::one()];
        let instance_columns = vec![public_input];
        let circuit = NoteCircuit::new(note);

        let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }
}
