use std::rc::Rc;

use halo2_base::halo2_proofs::{
    halo2curves::bn256::{self, Bn256, G1Affine},
    plonk::{create_proof, Circuit, ProvingKey},
    poly::{
        commitment::ParamsProver,
        kzg::{
            commitment::KZGCommitmentScheme,
            multiopen::{ProverSHPLONK, VerifierSHPLONK},
            strategy::AccumulatorStrategy,
        },
        VerificationStrategy,
    },
    transcript::TranscriptReadBuffer,
};
use rand::rngs::OsRng;
use snark_verifier::{
    loader::evm::EvmLoader,
    pcs::kzg::{Bdfg21, Kzg},
    system::halo2::transcript::evm::EvmTranscript,
    verifier::{Plonk, PlonkVerifier},
};

use crate::{data::ParameterSet, params::load_params, CircuitKind};

pub type Error = halo2_base::halo2_proofs::plonk::Error;

pub fn gen_proof<C: Circuit<bn256::Fr>>(
    params: ParameterSet,
    pk: &ProvingKey<bn256::G1Affine>,
    circuit: C,
    instances: &[&[bn256::Fr]],
) -> Result<Vec<u8>, crate::Error> {
    let params = load_params(params);

    let mut transcript: EvmTranscript<_, _, _, _> =
        halo2_base::halo2_proofs::transcript::TranscriptWriterBuffer::<_, G1Affine, _>::init(
            Vec::new(),
        );
    create_proof::<KZGCommitmentScheme<Bn256>, ProverSHPLONK<Bn256>, _, _, _, _>(
        params,
        pk,
        &[circuit],
        &[instances],
        OsRng,
        &mut transcript,
    )
    .map_err(crate::Error::err)?;

    Ok(transcript.finalize())
}

pub fn verify_proof(
    kind: CircuitKind,
    proof: &[u8],
    instances: &[Vec<bn256::Fr>],
) -> Result<bool, Error> {
    let params = load_params(kind.params());
    let vk = kind.vk();

    let mut transcript: EvmTranscript<_, _, _, _> =
        TranscriptReadBuffer::<_, G1Affine, _>::init(proof);

    Ok(VerificationStrategy::<_, VerifierSHPLONK<_>>::finalize(
        halo2_base::halo2_proofs::plonk::verify_proof::<
            _,
            VerifierSHPLONK<_>,
            _,
            EvmTranscript<_, _, _, _>,
            _,
        >(
            params.verifier_params(),
            vk,
            AccumulatorStrategy::new(params.verifier_params()),
            &[&instances.iter().map(|a| a.as_slice()).collect::<Vec<_>>()],
            &mut transcript,
        )?,
    ))
}

pub fn generate_verifier(
    params: ParameterSet,
    pk: &ProvingKey<bn256::G1Affine>,
    num_instance: Vec<usize>,
) -> String {
    let params = load_params(params);
    let vk = pk.get_vk();

    let svk: snark_verifier::pcs::kzg::KzgSuccinctVerifyingKey<G1Affine> = params.get_g()[0].into();
    let dk: snark_verifier::pcs::kzg::KzgDecidingKey<Bn256> = (params.g2(), params.s_g2()).into();
    let protocol = snark_verifier::system::halo2::compile(
        params,
        vk,
        snark_verifier::system::halo2::Config::kzg().with_num_instance(num_instance.clone()),
    );
    let loader: Rc<EvmLoader> = EvmLoader::new::<bn256::Fq, bn256::Fr>();
    let protocol = protocol.loaded(&loader);

    let mut transcript = EvmTranscript::<G1Affine, Rc<EvmLoader>, _, _>::new(&loader);
    let instances = transcript.load_instances(num_instance);
    let proof =
        Plonk::<Kzg<Bn256, Bdfg21>>::read_proof(&svk, &protocol, &instances, &mut transcript);
    Plonk::<Kzg<Bn256, Bdfg21>>::verify(&svk, &dk, &protocol, &instances, &proof);

    loader.yul_code()
}
