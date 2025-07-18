use crate::{AggUtxoProof, ToBytes};
use borsh::{BorshDeserialize, BorshSerialize};
use element::Element;
use hash::hash_merge;
use primitives::serde::{deserialize_base64, serialize_base64};
use serde::{Deserialize, Serialize};

/// The data required to prove an AggAgg transaction, this aggregates multiple AggUtxo proofs into
/// a single proof. Expects each new_root from the previous AggUtxo proof to be the same as the
/// old_root of the next AggUtxo proof.
#[derive(Debug, Clone)]
pub struct AggAgg {
    /// The proofs for the AggAgg transaction
    pub proofs: [AggUtxoProof; 2],
    /// Hash of the proofs being verified in agg_agg proof
    pub verification_key_hash: Element,
}

impl AggAgg {
    /// Create a new AggAgg
    #[must_use]
    pub fn new(proofs: [AggUtxoProof; 2], verification_key_hash: Element) -> Self {
        Self {
            proofs,
            verification_key_hash,
        }
    }

    /// Get the old root of the AggAgg transaction
    #[must_use]
    pub fn old_root(&self) -> Element {
        self.proofs[0].public_inputs.old_root
    }

    /// Get the new root of the AggAgg transaction
    #[must_use]
    pub fn new_root(&self) -> Element {
        self.proofs[1].public_inputs.new_root
    }

    /// Get the messages from the UTXO proofs
    #[must_use]
    pub fn messages(&self) -> Vec<Element> {
        self.proofs
            .iter()
            .flat_map(|p| p.public_inputs.messages)
            .collect::<Vec<_>>()
    }

    /// Get the public inputs for the AggAgg circuit
    #[must_use]
    pub fn public_inputs(&self) -> AggAggPublicInput {
        AggAggPublicInput {
            verification_key_hash: self.verification_key_hash,
            old_root: self.old_root(),
            new_root: self.new_root(),
            commit_hash: self.commit_hash(),
            messages: self.messages(),
        }
    }

    /// Commit hash of the agg_agg (will be posted onchain and verified by Celestia)
    #[must_use]
    pub fn commit_hash(&self) -> Element {
        hash_merge([
            self.proofs[0].public_inputs.commit_hash,
            self.proofs[1].public_inputs.commit_hash,
        ])
    }
}

/// Raw proof bytes for AggAgg proof (no public inputs)
#[derive(Debug, Default, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct AggAggProofBytes(
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    pub Vec<u8>,
);

/// The public input for a AggAgg transaction
#[derive(Default, Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct AggAggPublicInput {
    /// Verification key hash agg_utxo
    pub verification_key_hash: Element,
    /// The old root of the tree
    pub old_root: Element,
    /// The new root of the tree
    pub new_root: Element,
    /// Commit hash
    pub commit_hash: Element,
    /// The messages of the transactions
    pub messages: Vec<Element>,
}

impl AggAggPublicInput {
    /// Convert the AggAggPublicInput to a AggAggPublicInputBytes
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 * (1 + 1 + 1 + 1 + 2 * 18));

        bytes.extend(self.verification_key_hash.to_be_bytes());
        bytes.extend(self.old_root.to_be_bytes());
        bytes.extend(self.new_root.to_be_bytes());
        bytes.extend(self.commit_hash.to_be_bytes());

        for message in &self.messages {
            bytes.extend(message.to_be_bytes());
        }

        bytes
    }
}

/// The output proof for a AggAgg transaction
#[derive(Debug, Default, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct AggAggProof {
    /// The proof for the AggAgg transaction
    pub proof: AggAggProofBytes,
    /// The public input for the AggAgg transaction
    pub public_inputs: AggAggPublicInput,
    /// KZG accumulator inputs
    pub kzg: Vec<Element>,
}

impl ToBytes for AggAggProof {
    /// Convert the AggAggProof to a AggAggProofBytes
    #[must_use]
    fn to_bytes(&self) -> Vec<u8> {
        // TODO: move to impl detail of proving backend
        let pi = self.public_inputs.to_bytes();
        let kzg = self
            .kzg
            .iter()
            .flat_map(|e| e.to_be_bytes())
            .collect::<Vec<u8>>();
        let proof = &self.proof.0;
        let total_len = pi.len() + kzg.len() + proof.len();
        let total_len_fields = total_len / 32;
        [
            u32::try_from(total_len_fields)
                .unwrap()
                .to_be_bytes()
                .as_slice(),
            pi.as_slice(),
            kzg.as_slice(),
            proof.as_slice(),
        ]
        .concat()
    }
}
