use crate::{BbBackend, Result};
use element::Base;

pub async fn verify(
    bb_backend: &dyn BbBackend,
    key: &[u8],
    proof: &[u8],
    public_inputs: &[u8],
    oracle_hash_keccak: bool,
) -> Result<()> {
    bb_backend
        .verify(proof, public_inputs, key, oracle_hash_keccak)
        .await
}

#[derive(Debug, Clone)]
pub struct VerificationKeyHash(pub Base);

#[derive(Debug, Clone)]
pub struct VerificationKey(pub Vec<Base>);
