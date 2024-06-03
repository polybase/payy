use crate::chips::add::AddCulmChip;
use crate::chips::is_constant::IsConstantChip;
use crate::chips::swap::CondSwapChip;
use crate::data::{Note, ParameterSet, Points};
use crate::params::load_params;
use crate::util::keygen_from_params;
use crate::Snark;
use crate::{chips::poseidon::PoseidonConfig, util::assign_private_input};
use halo2_base::halo2_proofs::circuit::Value;
use halo2_base::halo2_proofs::halo2curves::bn256::G1Affine;
use halo2_base::halo2_proofs::plonk::{ProvingKey, VerifyingKey};
use halo2_base::halo2_proofs::{
    circuit::Layouter,
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, Error, Instance},
};
use smirk::hash_merge;
use zk_primitives::Element;

const NUM_NOTES: usize = 112;

impl Points {
    pub fn new(secret_key: Element, notes: Vec<Note>) -> Self {
        assert!(notes.len() == NUM_NOTES, "notes must be 112");
        Self { secret_key, notes }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        advice: Column<Advice>,
        instance: Column<Instance>,
        add_chip: AddCulmChip<Fr>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        swap_chip: CondSwapChip<Fr>,
        is_zero_chip: IsConstantChip<Fr>,
    ) -> Result<(), Error> {
        let unverified_address = assign_private_input(
            || "unverified address",
            layouter.namespace(|| "unverified address witness"),
            advice,
            Value::known(self.address()),
        )?;

        let mut values = vec![];

        for note in &self.notes {
            let note_cells = note.enforce_constraints(
                layouter.namespace(|| "input note enforce commitment"),
                advice,
                poseidon_config.clone(),
                is_zero_chip.clone(),
                swap_chip.clone(),
            )?;

            // Swap address if the note is padding
            let (address, _) = swap_chip.swap_assigned(
                layouter.namespace(|| "swap address?"),
                (&note_cells.address, &unverified_address),
                &note_cells.is_padding,
            )?;

            // Contrain address
            layouter.constrain_instance(address.cell(), instance, 0)?;

            // Add values so we can get total
            values.push(note_cells.value);
        }

        // Total sum of all notes
        let total = add_chip.assign(layouter.namespace(|| "totals"), values.as_slice())?;

        // Constrain total
        layouter.constrain_instance(total.cell(), instance, 1)?;

        Ok(())
    }

    pub fn address(&self) -> Fr {
        hash_merge([self.secret_key, Element::ZERO]).into()
    }

    pub fn total_value(&self) -> Element {
        self.notes.iter().map(|n| n.value).sum()
    }

    pub(crate) fn public_inputs(&self) -> Vec<Fr> {
        let mut inputs = vec![self.address(), self.total_value().to_base()];

        for note in &self.notes {
            inputs.push(note.commitment().into());
            inputs.push(note.nullifier(self.secret_key).into())
        }

        inputs
    }

    pub fn snark(&self, params: ParameterSet) -> Result<Snark, Error> {
        let (pk, _) = self.keygen(params);
        Snark::create(
            self.clone(),
            vec![self.public_inputs()],
            load_params(params),
            &pk,
        )
    }

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }
}

impl Default for Points {
    fn default() -> Self {
        Self {
            secret_key: Element::default(),
            notes: (0..112).map(|_| Note::padding_note()).collect(),
        }
    }
}
