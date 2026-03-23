use zk_primitives::{Signature, SignatureProof, SignaturePublicInput, get_address_for_private_key};

use crate::circuits::Proof;

use super::conversions::impl_circuit_proof_conversions;
use super::generated::signature::{
    SignatureInput, SignaturePublicInputs as CircuitSignaturePublicInputs,
};

impl From<Signature> for SignatureInput {
    fn from(signature: Signature) -> Self {
        let Signature {
            secret_key,
            message,
        } = signature;

        SignatureInput {
            owner_pk: secret_key,
            message_hash: hash::hash_merge([secret_key, message]),
            address: get_address_for_private_key(secret_key),
            message,
        }
    }
}

impl_circuit_proof_conversions!(
    SignaturePublicInput,
    CircuitSignaturePublicInputs,
    SignatureProof,
    Proof<CircuitSignaturePublicInputs>,
    [address, message]
);
