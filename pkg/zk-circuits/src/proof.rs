use std::io::Cursor;

use halo2_base::halo2_proofs::{
    circuit::Value,
    halo2curves::{
        bn256::{Bn256, Fr, G1Affine},
        pairing::Engine,
    },
    plonk::{self, Circuit, ProvingKey, VerifyingKey},
    poly::{
        commitment::ParamsProver,
        kzg::{
            commitment::{KZGCommitmentScheme, ParamsKZG},
            multiopen::VerifierSHPLONK,
        },
        kzg::{multiopen::ProverSHPLONK, strategy::SingleStrategy},
    },
    transcript::{Blake2bRead, Blake2bWrite, TranscriptReadBuffer, TranscriptWriterBuffer},
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof(Vec<u8>);

impl Proof {
    /// Creates a proof for the given circuits and instances.
    #[allow(dead_code)]
    pub fn create<C: Circuit<Fr>>(
        params: &ParamsKZG<Bn256>,
        pk: &ProvingKey<G1Affine>,
        circuit: C,
        instances: &[&[Fr]],
        rng: impl RngCore,
    ) -> Result<Self, plonk::Error> {
        let mut transcript = Blake2bWrite::<_, <Bn256 as Engine>::G1Affine, _>::init(Vec::new());
        plonk::create_proof::<KZGCommitmentScheme<Bn256>, ProverSHPLONK<Bn256>, _, _, _, _>(
            params,
            pk,
            &[circuit],
            &[instances],
            rng,
            &mut transcript,
        )?;
        Ok(Self(transcript.finalize()))
    }

    // TODO: this should be generic, as `create` above
    /// Verifies this proof with the given instances.
    #[allow(dead_code)]
    pub fn verify(
        &self,
        vk: &VerifyingKey<G1Affine>,
        params: &ParamsKZG<Bn256>,
        instances: &[&[Fr]],
    ) -> Result<(), plonk::Error> {
        let strategy = SingleStrategy::new(params);
        let mut transcript =
            Blake2bRead::<_, <Bn256 as Engine>::G1Affine, _>::init(Cursor::new(self.0.clone()));
        plonk::verify_proof::<_, VerifierSHPLONK<_>, _, _, _>(
            params.verifier_params(),
            vk,
            strategy,
            &[instances],
            &mut transcript,
        )
    }

    /// Constructs a new Proof value.
    pub fn new(bytes: Vec<u8>) -> Self {
        Proof(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn inner(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn value(&self) -> Value<&[u8]> {
        Value::known(self.as_bytes())
    }
}
