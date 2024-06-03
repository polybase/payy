use crate::data::{ParameterSet, Signature};
use crate::params::load_params;
use crate::proof::Proof;
use crate::util::{assign_constant, keygen_from_params};
use crate::Snark;
use crate::{
    chips::{
        poseidon::{poseidon_hash_gadget, PoseidonConfig},
        poseidon_hash,
    },
    util::assign_private_input,
};
use halo2_base::halo2_proofs::halo2curves::bn256::{Bn256, G1Affine};
use halo2_base::halo2_proofs::plonk::VerifyingKey;
use halo2_base::halo2_proofs::poly::kzg::commitment::ParamsKZG;
use halo2_base::halo2_proofs::{
    circuit::{Layouter, Value},
    halo2curves::bn256::Fr,
    plonk::{Advice, Column, Error, Instance, ProvingKey},
};
use rand::RngCore;
use smirk::Element;

impl Signature {
    pub fn new(secret_key: Element, message: Element) -> Self {
        Self {
            secret_key,
            message,
        }
    }

    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        advice: Column<Advice>,
        instance: Column<Instance>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
    ) -> Result<(), Error> {
        // Witness the message
        let message = assign_private_input(
            || "message",
            layouter.namespace(|| "message witness"),
            advice,
            Value::known(self.message.to_base()),
        )?;

        // Witness the message
        let secret_key = assign_private_input(
            || "secret_key",
            layouter.namespace(|| "secret key witness"),
            advice,
            Value::known(self.secret_key.to_base()),
        )?;

        let padding = assign_constant(
            || "padding witness",
            layouter.namespace(|| "padding witness"),
            advice,
            Fr::zero(),
        )?;

        let address_from_private_key = poseidon_hash_gadget(
            poseidon_config,
            layouter.namespace(|| "address from pk"),
            [secret_key, padding],
        )?;

        // Constrain address to be the same as verified address
        layouter.constrain_instance(address_from_private_key.cell(), instance, 0)?;

        // Constrain message witness
        layouter.constrain_instance(message.cell(), instance, 1)?;

        Ok(())
    }

    pub(crate) fn address(&self) -> Fr {
        poseidon_hash([self.secret_key.into(), Fr::zero()])
    }

    pub(crate) fn public_inputs(&self) -> Vec<Fr> {
        vec![self.address(), self.message.into()]
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

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }
}
