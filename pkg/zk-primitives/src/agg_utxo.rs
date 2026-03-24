// lint-long-file-override allow-max-lines=300
use crate::{MerklePath, UtxoProof};
use crate::{ProofBytes, UtxoKind, impl_serde_for_element_array};
use element::{Base, Element};
use hash::hash_merge;

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

    /// Get messages with compaction (trailing zeros after last exportable message)
    #[must_use]
    #[allow(clippy::large_stack_arrays)]
    pub fn messages(&self) -> [Element; 1000] {
        let mut messages = [Element::ZERO; 1000];
        let mut index = 0;

        for proof in &self.proofs {
            let proof_messages = match proof.utxo_proof.kind() {
                UtxoKind::Null | UtxoKind::Send => &[][..],
                UtxoKind::Mint => &proof.utxo_proof.public_inputs.messages[..4],
                UtxoKind::Burn => &proof.utxo_proof.public_inputs.messages[..],
            };

            for &message in proof_messages {
                messages[index] = message;
                index += 1;
            }
        }

        // Remaining elements are already zero (trailing zeros after compaction)
        messages
    }
}

/// A Utxo proof bundle with merkle proofs
#[derive(Debug, Clone, PartialEq, Eq)]
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
    fn default() -> UtxoProofBundleWithMerkleProofs {
        let merkle_path = MerklePath::default();
        Self {
            utxo_proof: UtxoProof::default(),
            input_merkle_paths: [merkle_path.clone(), merkle_path.clone()],
            output_merkle_paths: [merkle_path.clone(), merkle_path.clone()],
        }
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
    /// Verification key hashes for allowed proof types
    pub verification_key_hash: [Element; 2],
    /// The old root of the tree
    pub old_root: Element,
    /// The new root of the tree
    pub new_root: Element,
    /// Commit hash
    pub commit_hash: Element,
    /// The messages of the transactions (compacted with trailing zeros)
    pub messages: [Element; 1000],
}

impl Default for AggUtxoPublicInput {
    #[allow(clippy::large_stack_arrays)]
    fn default() -> Self {
        Self {
            verification_key_hash: [Element::ZERO; 2],
            old_root: Element::ZERO,
            new_root: Element::ZERO,
            commit_hash: Element::ZERO,
            messages: [Element::ZERO; 1000],
        }
    }
}

impl AggUtxoPublicInput {
    /// Check if this is a padding proof (if old_root is zero element)
    #[must_use]
    pub fn is_padding(&self) -> bool {
        self.old_root == Element::ZERO
    }
}

/// The output proof for a AggUtxo transaction
#[derive(Default, Debug, Clone)]
pub struct AggUtxoProof {
    /// The proof for the AggUtxo transaction
    pub proof: ProofBytes,
    /// The public input for the AggUtxo transaction
    pub public_inputs: AggUtxoPublicInput,
}
