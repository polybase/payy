use element::Element;
use primitives::serde::{deserialize_base64, serialize_base64};
use serde::{Deserialize, Serialize};

use crate::ToBytes;

/// A signature is a message signed by a secret key, often used to authenticate a user's
/// posession of a address
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Signature {
    /// Secret key for the address, required to spend a note
    pub secret_key: Element,
    /// Message to be signed
    pub message: Element,
}

impl Signature {
    /// Create a new signature
    #[must_use]
    pub fn new(secret_key: Element, message: Element) -> Self {
        Self {
            secret_key,
            message,
        }
    }

    /// Get the message hash
    #[must_use]
    pub fn message_hash(&self) -> Element {
        hash::hash_merge([self.secret_key, self.message])
    }
}

/// The output zk proof for a Signature circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureProofBytes(
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    pub Vec<u8>,
);

/// The public inputs for a signature proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignaturePublicInput {
    /// The address of the sender
    pub address: Element,
    /// The message to be signed
    pub message: Element,
}

impl SignaturePublicInput {
    /// Convert the public inputs to a bytes vector
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        [self.address.to_be_bytes(), self.message.to_be_bytes()].concat()
    }
}

/// The output proof for a signature proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignatureProof {
    /// The proof for the signature proof
    pub proof: SignatureProofBytes,
    /// The public inputs for the signature proof
    pub public_inputs: SignaturePublicInput,
}

impl SignatureProof {}

impl ToBytes for SignatureProof {
    /// Convert the signature proof to a bytes vector
    #[must_use]
    fn to_bytes(&self) -> Vec<u8> {
        // TODO: move to impl detail of proving backend
        let pi = self.public_inputs.to_bytes();
        let proof = self.proof.0.clone();
        let total_len = pi.len() + proof.len();
        let total_len_fields = total_len / 32;
        [
            u32::try_from(total_len_fields)
                .unwrap()
                .to_be_bytes()
                .as_slice(),
            pi.as_slice(),
            proof.as_slice(),
        ]
        .concat()
    }
}
