use element::Element;
use serde::{Deserialize, Serialize};

use crate::ProofBytes;

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

/// The public inputs for a signature proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignaturePublicInput {
    /// The address of the sender
    pub address: Element,
    /// The message to be signed
    pub message: Element,
}

/// The output proof for a signature proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignatureProof {
    /// The proof for the signature proof
    pub proof: ProofBytes,
    /// The public inputs for the signature proof
    pub public_inputs: SignaturePublicInput,
}
