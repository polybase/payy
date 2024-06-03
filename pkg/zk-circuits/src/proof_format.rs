use halo2_base::halo2_proofs::{
    halo2curves::bn256::{Fr, G1Affine},
    plonk::{verify_proof, VerifyingKey},
    poly::{
        commitment::ParamsProver,
        kzg::{multiopen::VerifierSHPLONK, strategy::AccumulatorStrategy},
        VerificationStrategy,
    },
    transcript::TranscriptWriterBuffer,
};
use std::io::Cursor;

use smirk::Element;
use snark_verifier::loader::native::NativeLoader;

use crate::{
    chips::aggregation::types::PoseidonTranscript,
    data::{ParameterSet, SnarkWitnessV1},
    keys::CircuitKind,
    params::load_params,
    Snark,
};

impl SnarkWitnessV1 {
    pub(crate) fn new(instances: Vec<Vec<Element>>, proof: Vec<u8>) -> Self {
        Self { instances, proof }
    }

    pub fn to_snark(&self, vk: &VerifyingKey<G1Affine>, params: ParameterSet) -> Snark {
        Snark::from_witness(self.clone(), vk, params)
    }

    pub(crate) fn fr_instances(&self) -> Vec<Vec<Fr>> {
        self.instances
            .iter()
            .map(|v| v.iter().map(|v| v.to_base()).collect())
            .collect()
    }

    pub fn verify(&self, kind: CircuitKind) -> bool {
        let params = load_params(kind.params());
        let vk = kind.vk();

        let mut transcript =
            PoseidonTranscript::<NativeLoader, _>::init(Cursor::new(self.proof.clone()));

        VerificationStrategy::<_, VerifierSHPLONK<_>>::finalize(
            verify_proof::<_, VerifierSHPLONK<_>, _, PoseidonTranscript<NativeLoader, _>, _>(
                params.verifier_params(),
                vk,
                AccumulatorStrategy::new(params.verifier_params()),
                &[&self
                    .fr_instances()
                    .iter()
                    .map(|v| v.as_slice())
                    .collect::<Vec<_>>()],
                &mut transcript,
            )
            .unwrap(),
        )
    }
}
