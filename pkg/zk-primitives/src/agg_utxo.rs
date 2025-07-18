use crate::{bytes_to_elements, impl_serde_for_element_array, ToBytes};
use crate::{MerklePath, UtxoProof};
use element::{Base, Element};
use hash::hash_merge;
use primitives::serde::{deserialize_base64, serialize_base64};
use serde::{Deserialize, Serialize};

/// The data required to prove an AggUtxo transaction, this aggregates multiple Utxo proofs into
/// a single proof. It also validates that the input notes are removed from the tree and the output
/// notes are added to the tree.
#[derive(Debug, Clone)]
pub struct AggUtxo {
    /// The proofs for the AggUtxo transaction
    pub proofs: [UtxoProofBundleWithMerkleProofs; 3],
    /// The old root of the tree (must match the first merkle proof)
    pub old_root: Element,
    /// The new root of the tree (must match the last merkle proof)
    pub new_root: Element,
}

impl AggUtxo {
    /// Create a new AggUtxo
    #[must_use]
    pub fn new(
        proofs: [UtxoProofBundleWithMerkleProofs; 3],
        old_root: Element,
        new_root: Element,
    ) -> Self {
        Self {
            proofs,
            old_root,
            new_root,
        }
    }

    /// Commit hash for utxo_agg
    #[must_use]
    pub fn commit_hash(&self) -> Element {
        hash_merge([
            self.proofs[0].utxo_proof.public_inputs.commit_hash(),
            self.proofs[1].utxo_proof.public_inputs.commit_hash(),
            self.proofs[2].utxo_proof.public_inputs.commit_hash(),
        ])
    }
}

/// A Utxo proof bundle with merkle proofs
#[derive(Debug, Clone)]
pub struct UtxoProofBundleWithMerkleProofs {
    /// The proof for the Utxo
    pub utxo_proof: UtxoProof,
    /// The merkle path proofs for removing input notes
    pub input_merkle_paths: [MerklePath<161>; 2],
    /// The merkle path proofs for adding output notes
    pub output_merkle_paths: [MerklePath<161>; 2],
}

impl UtxoProofBundleWithMerkleProofs {
    /// Create a new UtxoProofBundleWithMerkleProofs
    #[must_use]
    pub fn new(utxo_proof: UtxoProof, merkle_paths: &[MerklePath<161>; 4]) -> Self {
        Self {
            utxo_proof,
            input_merkle_paths: [merkle_paths[0].clone(), merkle_paths[1].clone()],
            output_merkle_paths: [merkle_paths[2].clone(), merkle_paths[3].clone()],
        }
    }
}

impl Default for UtxoProofBundleWithMerkleProofs {
    /// Create a padding UtxoProofBundleWithMerkleProofs
    #[must_use]
    fn default() -> UtxoProofBundleWithMerkleProofs {
        let merkle_path = MerklePath::default();
        Self {
            utxo_proof: UtxoProof::default(),
            input_merkle_paths: [merkle_path.clone(), merkle_path.clone()],
            output_merkle_paths: [merkle_path.clone(), merkle_path.clone()],
        }
    }
}

/// Raw proof bytes for AggUtxo proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggUtxoProofBytes(
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    pub Vec<u8>,
);

impl AggUtxoProofBytes {
    /// Convert the AggUtxoProofBytes to a AggUtxoProofFields
    #[must_use]
    pub fn to_fields(&self) -> Vec<Element> {
        bytes_to_elements(&self.0)
    }
}

/// The proof for a Utxo transaction
#[derive(Debug, Clone)]
pub struct AggUtxoProofFields(pub [Element; 93]);
impl_serde_for_element_array!(AggUtxoProofFields, 93);

impl From<[Base; 93]> for AggUtxoProofFields {
    fn from(elements: [Base; 93]) -> Self {
        AggUtxoProofFields(elements.map(Element::from_base))
    }
}

impl From<&AggUtxoProofFields> for [Base; 93] {
    fn from(value: &AggUtxoProofFields) -> Self {
        value.0.map(|e| e.to_base())
    }
}

/// The public input for a AggUtxo transaction
#[derive(Debug, Clone)]
pub struct AggUtxoPublicInput {
    /// The messages of the transactions
    pub messages: [Element; 18],
    /// The old root of the tree
    pub old_root: Element,
    /// The new root of the tree
    pub new_root: Element,
    /// Commit hash
    pub commit_hash: Element,
}

impl AggUtxoPublicInput {
    /// Convert the AggUtxoPublicInput to a AggUtxoPublicInputBytes
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        [
            self.messages[0].to_be_bytes(),
            self.messages[1].to_be_bytes(),
            self.messages[2].to_be_bytes(),
            self.messages[3].to_be_bytes(),
            self.messages[4].to_be_bytes(),
            self.messages[5].to_be_bytes(),
            self.messages[6].to_be_bytes(),
            self.messages[7].to_be_bytes(),
            self.messages[8].to_be_bytes(),
            self.messages[9].to_be_bytes(),
            self.messages[10].to_be_bytes(),
            self.messages[11].to_be_bytes(),
            self.messages[12].to_be_bytes(),
            self.messages[13].to_be_bytes(),
            self.messages[14].to_be_bytes(),
            self.messages[15].to_be_bytes(),
            self.messages[16].to_be_bytes(),
            self.messages[17].to_be_bytes(),
            self.old_root.to_be_bytes(),
            self.new_root.to_be_bytes(),
            self.commit_hash.to_be_bytes(),
        ]
        .concat()
    }
}

/// The output proof for a AggUtxo transaction
#[derive(Debug, Clone)]
pub struct AggUtxoProof {
    /// The proof for the AggUtxo transaction
    pub proof: AggUtxoProofBytes,
    /// The public input for the AggUtxo transaction
    pub public_inputs: AggUtxoPublicInput,
}

impl ToBytes for AggUtxoProof {
    /// Convert the UtxoProof to a UtxoProofFields
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
