use crate::{Note, ProofBytes};
use element::Element;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

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

/// Public input for the points circuit
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
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

/// Bundle of a proof and its public inputs for points. This can be used
/// verify the proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct PointsProof {
    /// Proof
    pub proof: ProofBytes,
    /// Public inputs
    pub public_inputs: PointsPublicInput,
}
