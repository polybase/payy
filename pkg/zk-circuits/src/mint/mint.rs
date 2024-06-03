use crate::chips::swap::CondSwapChip;
use crate::chips::{is_constant::IsConstantChip, poseidon::PoseidonConfig};
use crate::data::{Mint, Note, ParameterSet};
use crate::params::load_params;
use crate::proof::Proof;
use crate::util::keygen_from_params;
use crate::{evm_verifier, Snark};
use halo2_base::halo2_proofs::halo2curves::bn256::{Bn256, G1Affine};
use halo2_base::halo2_proofs::plonk::VerifyingKey;
use halo2_base::halo2_proofs::poly::kzg::commitment::ParamsKZG;
use halo2_base::halo2_proofs::{
    circuit::Layouter,
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, Error, Instance, ProvingKey},
};
use rand::RngCore;

impl<const L: usize> Mint<L> {
    pub fn new(notes: [Note; L]) -> Self {
        Mint { notes }
    }

    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        instance: Column<Instance>,
        advice: Column<Advice>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        is_zero_chip: IsConstantChip<Fr>,
        swap_chip: CondSwapChip<Fr>,
    ) -> Result<(), Error> {
        for (i, note) in self.notes.iter().enumerate() {
            // Ensure note is of valid construction
            let note_cells = note.enforce_constraints(
                layouter.namespace(|| format!("note {i}")),
                advice,
                poseidon_config.clone(),
                is_zero_chip.clone(),
                swap_chip.clone(),
            )?;

            // Constrain note details to public instances
            layouter.constrain_instance(note_cells.cm.cell(), instance, i * 3)?;
            layouter.constrain_instance(note_cells.value.cell(), instance, (i * 3) + 1)?;
            layouter.constrain_instance(note_cells.source.cell(), instance, (i * 3) + 2)?;
        }

        Ok(())
    }

    pub fn public_inputs(&self) -> Vec<Fr> {
        let mut inputs = vec![];

        for note in self.notes.iter() {
            // Expose the note details we need to verify in Ethereum
            inputs.push(note.commitment().into());
            inputs.push(note.value().into());
            inputs.push(note.source().into());
        }

        inputs
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

    pub fn snark(&self, params: ParameterSet) -> Result<Snark, Error> {
        let (pk, _) = self.keygen(params);

        Snark::create(
            self.clone(),
            vec![self.public_inputs()],
            load_params(params),
            &pk,
        )
    }

    pub fn evm_proof(&self, params: ParameterSet) -> Result<Vec<u8>, crate::Error> {
        let (pk, _) = self.keygen(params);

        evm_verifier::gen_proof(params, &pk, self.clone(), &[&self.public_inputs()])
    }

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }
}
