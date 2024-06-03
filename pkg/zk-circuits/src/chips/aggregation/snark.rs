use crate::{
    data::{ParameterSet, SnarkWitnessV1},
    params::load_params,
};

use super::types::{PoseidonTranscript, Svk};
use halo2_base::halo2_proofs::{
    circuit::Value,
    halo2curves::bn256::{Bn256, Fr, G1Affine},
    plonk::{create_proof, Circuit, Error, ProvingKey, VerifyingKey},
    poly::{
        commitment::ParamsProver,
        kzg::{
            commitment::{KZGCommitmentScheme, ParamsKZG},
            multiopen::ProverSHPLONK,
        },
    },
    transcript::TranscriptWriterBuffer,
};
use itertools::Itertools;
use rand::rngs::OsRng;
use snark_verifier::{
    loader::native::NativeLoader,
    pcs::kzg::KzgDecidingKey,
    system::halo2::{compile, Config},
    Protocol,
};

#[derive(Clone, Debug)]
pub struct Snark {
    pub protocol: Protocol<G1Affine>,
    // TODO: make instances fixed size/typed
    pub instances: Vec<Vec<Fr>>,
    pub proof: Vec<u8>,
    pub svk: Svk,
    pub dk: KzgDecidingKey<Bn256>,
}

impl Snark {
    pub fn new(
        protocol: Protocol<G1Affine>,
        instances: Vec<Vec<Fr>>,
        proof: Vec<u8>,
        params: &ParamsKZG<Bn256>,
    ) -> Self {
        Self {
            protocol,
            instances,
            proof,
            svk: params.get_g()[0].into(),
            dk: (params.g2(), params.s_g2()).into(),
        }
    }

    pub fn create<C: Circuit<Fr>>(
        circuit: C,
        instances: Vec<Vec<Fr>>,
        params: &ParamsKZG<Bn256>,
        pk: &ProvingKey<G1Affine>,
    ) -> Result<Snark, Error> {
        let num_instance = instances.iter().map(|v| v.len()).collect_vec();

        let protocol = compile(
            params,
            pk.get_vk(),
            Config::kzg().with_num_instance(num_instance),
        );
        let slice = &instances
            .iter()
            .map(|instances| instances.as_slice())
            .collect_vec();

        let proof = {
            let mut transcript = PoseidonTranscript::<NativeLoader, _>::init(Vec::new());
            create_proof::<KZGCommitmentScheme<Bn256>, ProverSHPLONK<_>, _, _, _, _>(
                params,
                pk,
                &[circuit],
                &[slice.as_slice()],
                OsRng,
                &mut transcript,
            )
            .unwrap();
            transcript.finalize()
        };

        Ok(Self::new(protocol, instances.clone(), proof, params))
    }

    pub fn from_witness(
        witness: SnarkWitnessV1,
        vk: &VerifyingKey<G1Affine>,
        params: ParameterSet,
    ) -> Self {
        let params = load_params(params);

        let num_instance = witness.instances.iter().map(|v| v.len()).collect_vec();
        // let vk = keygen_vk(params, circuit).expect("keygen_vk should not fail");
        let protocol = compile(params, vk, Config::kzg().with_num_instance(num_instance));

        Self {
            protocol,
            instances: witness.fr_instances(),
            proof: witness.proof,
            svk: params.get_g()[0].into(),
            dk: (params.g2(), params.s_g2()).into(),
        }
    }

    pub fn to_witness(&self) -> SnarkWitnessV1 {
        SnarkWitnessV1::new(
            self.instances
                .iter()
                .map(|v| v.iter().map(|v| (*v).into()).collect_vec())
                .collect_vec(),
            self.proof.clone(),
        )
    }

    pub fn proof(&self) -> &[u8] {
        &self.proof
    }

    pub fn proof_value(&self) -> Value<&[u8]> {
        Value::known(&self.proof)
    }
}
