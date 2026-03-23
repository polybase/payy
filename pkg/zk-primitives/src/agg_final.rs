use crate::{AggAggProof, OracleProofBytes, ToBytes};
use borsh::{BorshDeserialize, BorshSerialize};
use element::Element;
use serde::{Deserialize, Serialize};

/// The data required to prove an AggFinal transaction, this takes a single AggAgg proof
/// and produces a final proof suitable for submission to the smart contract.
#[derive(Debug, Clone)]
pub struct AggFinal {
    /// The proof for the AggFinal transaction
    pub proof: AggAggProof,
}

impl AggFinal {
    /// Create a new AggFinal
    #[must_use]
    pub fn new(proof: AggAggProof) -> Self {
        Self { proof }
    }

    /// Get the old root of the AggFinal transaction
    #[must_use]
    pub fn old_root(&self) -> Element {
        self.proof.public_inputs.old_root
    }

    /// Get the new root of the AggFinal transaction
    #[must_use]
    pub fn new_root(&self) -> Element {
        self.proof.public_inputs.new_root
    }

    /// Get the messages from the proof
    #[must_use]
    pub fn messages(&self) -> Vec<Element> {
        self.proof.public_inputs.messages.to_vec()
    }

    /// Get the commit hash from the proof
    #[must_use]
    pub fn commit_hash(&self) -> Element {
        self.proof.public_inputs.commit_hash
    }

    /// Get the public inputs for the AggFinal circuit
    #[must_use]
    pub fn public_inputs(&self) -> AggFinalPublicInput {
        AggFinalPublicInput {
            old_root: self.old_root(),
            new_root: self.new_root(),
            commit_hash: self.commit_hash(),
            messages: self.messages(),
        }
    }
}

/// The public input for a AggFinal transaction
#[derive(Default, Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct AggFinalPublicInput {
    /// The old root of the tree
    pub old_root: Element,
    /// The new root of the tree
    pub new_root: Element,
    /// Commit hash
    pub commit_hash: Element,
    /// The messages of the transactions
    pub messages: Vec<Element>,
}

impl AggFinalPublicInput {
    /// Convert the AggFinalPublicInput to bytes
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 * (1 + 1 + 1 + self.messages.len()));

        bytes.extend(self.old_root.to_be_bytes());
        bytes.extend(self.new_root.to_be_bytes());
        bytes.extend(self.commit_hash.to_be_bytes());

        for message in &self.messages {
            bytes.extend(message.to_be_bytes());
        }

        bytes
    }
}

/// The output proof for a AggFinal transaction
#[derive(Debug, Default, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct AggFinalProof {
    /// The proof for the AggFinal transaction
    pub proof: OracleProofBytes,
    /// The public input for the AggFinal transaction
    pub public_inputs: AggFinalPublicInput,
    /// KZG accumulator inputs
    pub kzg: Vec<Element>,
}

impl ToBytes for AggFinalProof {
    /// Convert the AggFinalProof to bytes
    fn to_bytes(&self) -> Vec<u8> {
        // TODO: move to impl detail of proving backend
        let pi = self.public_inputs.to_bytes();
        let kzg = self
            .kzg
            .iter()
            .flat_map(|e| e.to_be_bytes())
            .collect::<Vec<u8>>();
        let proof = &self.proof.0;
        [pi.as_slice(), kzg.as_slice(), proof.as_slice()].concat()
    }
}
