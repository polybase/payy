use acvm::AcirField;
use async_trait::async_trait;
use element::Base;

use crate::{Result, Verify, circuits::proc_macro_interface::PublicInputs, verify::verify};

#[cfg(test)]
mod tests;

const FIELD_BYTES: usize = 32;

pub struct Proof<PublicInputs> {
    pub proof: Vec<u8>,
    pub public_inputs: PublicInputs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofDecodeError {
    PublicInputLengthOverflow { field_count: usize },
    PublicInputsTooShort { actual: usize, required: usize },
    LengthNotMultipleOfField { actual: usize },
}

impl ProofDecodeError {
    pub const fn kind(&self) -> ProofDecodeErrorKind {
        match self {
            Self::PublicInputLengthOverflow { .. } => {
                ProofDecodeErrorKind::PublicInputLengthOverflow
            }
            Self::PublicInputsTooShort { .. } => ProofDecodeErrorKind::PublicInputsTooShort,
            Self::LengthNotMultipleOfField { .. } => ProofDecodeErrorKind::LengthNotMultipleOfField,
        }
    }
}

impl std::fmt::Display for ProofDecodeError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PublicInputLengthOverflow { field_count } => {
                write!(
                    fmt,
                    "proof public input byte length overflow for {field_count} fields"
                )
            }
            Self::PublicInputsTooShort { actual, required } => write!(
                fmt,
                "proof bytes too short for public inputs: got {actual}, need at least {required}"
            ),
            Self::LengthNotMultipleOfField { actual } => {
                write!(
                    fmt,
                    "proof bytes length must be a multiple of 32: got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for ProofDecodeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofDecodeErrorKind {
    PublicInputLengthOverflow,
    PublicInputsTooShort,
    LengthNotMultipleOfField,
}

impl<P: PublicInputs> Proof<P> {
    pub fn try_from_raw_proof_bytes(
        raw_proof: Vec<u8>,
    ) -> std::result::Result<Self, ProofDecodeError> {
        let public_inputs_len = P::FIELD_COUNT.checked_mul(FIELD_BYTES).ok_or(
            ProofDecodeError::PublicInputLengthOverflow {
                field_count: P::FIELD_COUNT,
            },
        )?;
        if raw_proof.len() < public_inputs_len {
            return Err(ProofDecodeError::PublicInputsTooShort {
                actual: raw_proof.len(),
                required: public_inputs_len,
            });
        }
        if !raw_proof.len().is_multiple_of(FIELD_BYTES) {
            return Err(ProofDecodeError::LengthNotMultipleOfField {
                actual: raw_proof.len(),
            });
        }

        let public_inputs_bytes = &raw_proof[..public_inputs_len];
        let proof = raw_proof[public_inputs_len..].to_vec();
        let fields = bytes_to_bases(public_inputs_bytes);
        let mut iter = fields.into_iter();
        let public_inputs = P::from_fields(&mut iter);
        debug_assert!(iter.next().is_none(), "iterator should be exhausted");

        Ok(Self {
            proof,
            public_inputs,
        })
    }
}

fn bytes_to_bases(bytes: &[u8]) -> Vec<Base> {
    bytes
        .chunks_exact(FIELD_BYTES)
        .map(|chunk| {
            let mut arr = [0u8; FIELD_BYTES];
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
