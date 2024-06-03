use crate::chips::poseidon::poseidon_hash_gadget;
use crate::chips::swap::CondSwapChip;
use crate::chips::{is_constant::IsConstantChip, poseidon::PoseidonConfig};
use crate::data::{Burn, Note, ParameterSet};
use crate::evm_verifier;
use crate::util::{assign_constant, assign_private_input, keygen_from_params};
use halo2_base::halo2_proofs::circuit::Value;
use halo2_base::halo2_proofs::halo2curves::bn256::G1Affine;
use halo2_base::halo2_proofs::plonk::VerifyingKey;
use halo2_base::halo2_proofs::{
    circuit::Layouter,
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, Error, Instance, ProvingKey},
};
use smirk::{hash_merge, Element};

#[cfg(test)]
use halo2_base::halo2_proofs::halo2curves::bn256::Bn256;
#[cfg(test)]
use halo2_base::halo2_proofs::poly::kzg::commitment::ParamsKZG;

#[cfg(test)]
use crate::proof::Proof;
#[cfg(test)]
use rand::RngCore;

impl<const L: usize> Burn<L> {
    pub(crate) fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        instance: Column<Instance>,
        advice: Column<Advice>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        is_zero_chip: IsConstantChip<Fr>,
        swap_chip: CondSwapChip<Fr>,
    ) -> Result<(), Error> {
        // Witness to address
        let to_address = assign_private_input(
            || "to address witness",
            layouter.namespace(|| "to_address witness"),
            advice,
            Value::known(self.to_address.to_base()),
        )?;

        let zero = assign_constant(
            || "zero witness",
            layouter.namespace(|| "zero witness"),
            advice,
            Fr::zero(),
        )?;

        layouter.constrain_instance(to_address.cell(), instance, 0)?;

        // Witness secret_key
        let secret_key: halo2_base::halo2_proofs::circuit::AssignedCell<Fr, Fr> =
            assign_private_input(
                || "secret key witness",
                layouter.namespace(|| "secret key witness"),
                advice,
                Value::known(self.secret_key.to_base()),
            )?;

        for (i, note) in self.notes.iter().enumerate() {
            // Ensure note is of valid construction
            let note_cells = note.enforce_constraints(
                layouter.namespace(|| format!("input_note {i}")),
                advice,
                poseidon_config.clone(),
                is_zero_chip.clone(),
                swap_chip.clone(),
            )?;

            // Generate the nullifier
            let nullifier = poseidon_hash_gadget(
                poseidon_config.clone(),
                layouter.namespace(|| "nullifer hash"),
                [
                    note_cells.cm.clone(),
                    secret_key.clone(),
                    note_cells.psi.clone(),
                    zero.clone(),
                ],
            )?;

            // Constrain note details to public instances
            layouter.constrain_instance(nullifier.cell(), instance, i * 4 + 1)?;
            layouter.constrain_instance(note_cells.value.cell(), instance, (i * 4) + 2)?;
            layouter.constrain_instance(note_cells.source.cell(), instance, (i * 4) + 3)?;

            let sig = poseidon_hash_gadget(
                poseidon_config.clone(),
                layouter.namespace(|| "sig hash"),
                [
                    nullifier.clone(),
                    secret_key.clone(),
                    to_address.clone(),
                    zero.clone(),
                ],
            )?;

            layouter.constrain_instance(sig.cell(), instance, (i * 4) + 4)?;
        }

        Ok(())
    }

    pub fn signature(&self, note: &Note) -> Element {
        hash_merge([
            note.nullifier(self.secret_key),
            self.secret_key,
            self.to_address,
            // Padding
            Element::ZERO,
        ])
    }

    pub(crate) fn public_inputs(&self) -> Vec<Fr> {
        let mut inputs = vec![];

        // Address of request
        inputs.push(self.to_address.to_base());

        for note in self.notes.iter() {
            // Expose the note details we need to verify in Ethereum
            inputs.push(note.nullifier(self.secret_key).into());
            inputs.push(note.value().into());
            inputs.push(note.source().into());
            inputs.push(self.signature(note).into());
        }

        inputs
    }

    #[cfg(test)]
    pub(crate) fn prove(
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

    pub fn evm_proof(&self, params: ParameterSet) -> Result<Vec<u8>, crate::Error> {
        let (pk, _) = self.keygen(params);

        evm_verifier::gen_proof(params, &pk, self.clone(), &[&self.public_inputs()])
    }

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }
}
