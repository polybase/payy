use acvm::AcirField;
use async_trait::async_trait;
use element::Base;

use crate::{Result, Verify, circuits::proc_macro_interface::PublicInputs, verify::verify};

pub struct Proof<PublicInputs> {
    pub proof: Vec<u8>,
    pub public_inputs: PublicInputs,
}

impl<P: PublicInputs> Proof<P> {
    pub fn from_raw_proof_bytes(raw_proof: Vec<u8>) -> Self {
        assert!(
            raw_proof.len() >= P::FIELD_COUNT * 32,
            "proof bytes too short for public inputs"
        );
        assert!(
            raw_proof.len().is_multiple_of(32),
            "proof bytes length must be a multiple of 32"
        );

        let public_inputs_bytes = &raw_proof[..P::FIELD_COUNT * 32];
        let proof = raw_proof[P::FIELD_COUNT * 32..].to_vec();
        let fields = bytes_to_bases(public_inputs_bytes);
        let mut iter = fields.into_iter();
        let public_inputs = P::from_fields(&mut iter);
        debug_assert!(iter.next().is_none(), "iterator should be exhausted");

        Self {
            proof,
            public_inputs,
        }
    }
}

fn bytes_to_bases(bytes: &[u8]) -> Vec<Base> {
    assert!(
        bytes.len().is_multiple_of(32),
        "public input bytes length must be a multiple of 32"
    );
    bytes
        .chunks_exact(32)
        .map(|chunk| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(chunk);
            Base::from_be_bytes_reduce(&arr)
        })
        .collect()
}

#[async_trait]
impl<P: PublicInputs> Verify for Proof<P>
where
    Proof<P>: Send + Sync,
{
    async fn verify(&self, bb_backend: &dyn barretenberg_interface::BbBackend) -> Result<()> {
        let mut fields = vec![];
        self.public_inputs.to_fields(&mut fields);
        let public_inputs = fields
            .into_iter()
            .flat_map(|f| f.to_be_bytes())
            .collect::<Vec<_>>();
        verify(
            bb_backend,
            P::KEY,
            &self.proof,
            &public_inputs,
            P::ORACLE_HASH_KECCAK,
        )
        .await
    }
}
