use crate::{Note, ToBytes};
use element::Element;
use primitives::serde::{deserialize_base64, serialize_base64};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Points represents the data for a circuit that proves the number of points to give
/// to a user based on the notes they own.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Points {
    /// Secret key
    pub secret_keys: [Element; 10],
    /// Note values to be proven
    pub notes: [Note; 10],
    /// Timestamp
    pub timestamp: Element,
    /// Address to award points to
    pub address: Element,
}

impl Points {
    /// Create a new points proof
    #[must_use]
    pub fn new(address: Element, secret_keys: [Element; 10], notes: [Note; 10]) -> Self {
        Self {
            address,
            secret_keys,
            notes,
            timestamp: Element::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        }
    }

    /// Get the value of the points
    #[must_use]
    pub fn value(&self) -> Element {
        self.notes.iter().map(|note| note.value).sum()
    }

    /// Get the hash of the address and timestamp
    #[must_use]
    pub fn hash(&self) -> Element {
        hash::hash_merge([self.timestamp, self.address])
    }
}

/// The output zk proof for a Points circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsProofBytes(
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    pub Vec<u8>,
);

/// Public input for the points circuit
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PointsPublicInput {
    /// Value of the points
    pub value: Element,
    /// Timestamp (points proofs are valid for 1 day)
    pub timestamp: Element,
    /// Hash of address and timestamp
    pub hash: Element,
    /// Commitments to check that are in the tree and so points can be claimed
    pub commitments: [Element; 10],
}

impl PointsPublicInput {
    /// Convert the public input to bytes
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());
        bytes.extend_from_slice(&self.value.to_be_bytes());
        bytes.extend_from_slice(&self.hash.to_be_bytes());
        for commitment in &self.commitments {
            bytes.extend_from_slice(&commitment.to_be_bytes());
        }
        bytes
    }
}

/// Bundle of a proof and its public inputs for points. This can be used
/// verify the proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PointsProof {
    /// Proof
    pub proof: PointsProofBytes,
    /// Public inputs
    pub public_inputs: PointsPublicInput,
}

impl ToBytes for PointsProof {
    /// Convert the proof to bytes
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
