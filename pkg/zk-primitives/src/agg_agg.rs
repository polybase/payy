// lint-long-file-override allow-max-lines=400
use crate::{AggUtxoProof, ProofBytes, ToBytes};
use borsh::{BorshDeserialize, BorshSerialize};
use element::Element;
use hash::hash_merge;
use serde::{Deserialize, Serialize};

/// Enum representing a proof that can be either an AggUtxo or AggAgg proof
/// This enables composable aggregation where AggAgg can aggregate AggAgg proofs
#[derive(Debug, Clone)]
pub enum AggProof {
    /// An aggregated UTXO proof
    AggUtxo(Box<AggUtxoProof>),
    /// An aggregated AggAgg proof (recursive aggregation)
    AggAgg(Box<AggAggProof>),
}

impl AggProof {
    /// Get the old root of the proof
    #[must_use]
    pub fn old_root(&self) -> Element {
        match self {
            AggProof::AggUtxo(proof) => proof.public_inputs.old_root,
            AggProof::AggAgg(proof) => proof.public_inputs.old_root,
        }
    }

    /// Get the new root of the proof
    #[must_use]
    pub fn new_root(&self) -> Element {
        match self {
            AggProof::AggUtxo(proof) => proof.public_inputs.new_root,
            AggProof::AggAgg(proof) => proof.public_inputs.new_root,
        }
    }

    /// Get the commit hash of the proof
    #[must_use]
    pub fn commit_hash(&self) -> Element {
        match self {
            AggProof::AggUtxo(proof) => proof.public_inputs.commit_hash,
            AggProof::AggAgg(proof) => proof.public_inputs.commit_hash,
        }
    }

    /// Get the messages from the proof
    #[must_use]
    pub fn messages(&self) -> &[Element; 1000] {
        match self {
            AggProof::AggUtxo(proof) => &proof.public_inputs.messages,
            AggProof::AggAgg(proof) => &proof.public_inputs.messages,
        }
    }

    /// Check if this is a padding proof
    #[must_use]
    pub fn is_padding(&self) -> bool {
        match self {
            AggProof::AggUtxo(proof) => proof.public_inputs.is_padding(),
            AggProof::AggAgg(proof) => proof.public_inputs.old_root == Element::ZERO,
        }
    }

    /// Get the proof bytes
    #[must_use]
    pub fn proof_bytes(&self) -> &[u8] {
        match self {
            AggProof::AggUtxo(proof) => &proof.proof.0,
            AggProof::AggAgg(proof) => &proof.proof.0,
        }
    }

    /// Get the verification key hash
    #[must_use]
    pub fn verification_key_hash(&self) -> &[Element; 2] {
        match self {
            AggProof::AggUtxo(proof) => &proof.public_inputs.verification_key_hash,
            AggProof::AggAgg(proof) => &proof.public_inputs.verification_key_hash,
        }
    }
}

/// The data required to prove an AggAgg transaction, this aggregates multiple AggUtxo or AggAgg
/// proofs into a single proof. Expects each new_root from the previous proof to be the same as the
/// old_root of the next proof. This enables composable/recursive aggregation.
#[derive(Debug, Clone)]
pub struct AggAgg {
    /// The proofs for the AggAgg transaction (can be AggUtxo or AggAgg proofs)
    pub proofs: [AggProof; 2],
}

impl AggAgg {
    /// Create a new AggAgg
    #[must_use]
    pub fn new(proofs: [AggProof; 2]) -> Self {
        Self { proofs }
    }

    /// Get the old root of the AggAgg transaction
    #[must_use]
    pub fn old_root(&self) -> Element {
        self.proofs[0].old_root()
    }

    /// Get the new root of the AggAgg transaction
    #[must_use]
    pub fn new_root(&self) -> Element {
        if self.proofs[1].is_padding() {
            self.proofs[0].new_root()
        } else {
            self.proofs[1].new_root()
        }
    }

    /// Helper function to iterate through messages from proofs
    fn iterate_messages<F>(&self, mut callback: F)
    where
        F: FnMut(Element),
    {
        for proof in &self.proofs {
            // Exportable message kinds (2, 3 and 4) consume the next x messages, so when checking
            // for the end we skip the consumed messages, so we can find the first non-exportable kind.
            // Assumes that agg_utxo proof also compacts exportable kinds from index 0 (without gaps).
            let mut next_check = 0;

            for (j, &proof_message) in proof.messages().iter().enumerate() {
                // Update next checkpoint (or end)
                if next_check == j {
                    match proof_message {
                        element if element == Element::from(2u64) => {
                            // Mint
                            next_check += 4;
                        }
                        element
                            if element == Element::from(3u64) || element == Element::from(4u64) =>
                        {
                            // Burn, Swap
                            next_check += 5;
                        }
                        _ => break,
                    }
                }

                callback(proof_message);
            }
        }
    }

    /// Get the count of messages from the proofs
    #[must_use]
    pub fn messages_count(&self) -> usize {
        let mut messages_count = 0;

        self.iterate_messages(|_| {
            messages_count += 1;
        });

        messages_count
    }

    /// Get the messages from the proofs with compaction (trailing zeros)
    #[must_use]
    #[allow(clippy::large_stack_arrays)]
    pub fn messages(&self) -> [Element; 1000] {
        let mut messages = [Element::ZERO; 1000];
        let mut messages_index = 0;

        self.iterate_messages(|proof_message| {
            messages[messages_index] = proof_message;
            messages_index += 1;
        });

        messages
    }

    /// Get the public inputs for the AggAgg circuit
    #[must_use]
    pub fn public_inputs(&self) -> AggAggPublicInput {
        AggAggPublicInput {
            verification_key_hash: *self.proofs[0].verification_key_hash(),
            old_root: self.old_root(),
            new_root: self.new_root(),
            commit_hash: self.commit_hash(),
            messages: self.messages(),
        }
    }

    /// Commit hash of the agg_agg (will be posted onchain and verified by Celestia)
    #[must_use]
    pub fn commit_hash(&self) -> Element {
        hash_merge([self.proofs[0].commit_hash(), self.proofs[1].commit_hash()])
    }
}

/// The public input for a AggAgg transaction
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct AggAggPublicInput {
    /// Verification key hashes for allowed proof types (AggUtxo and AggAgg)
    pub verification_key_hash: [Element; 2],
    /// The old root of the tree
    pub old_root: Element,
    /// The new root of the tree
    pub new_root: Element,
    /// Commit hash
    pub commit_hash: Element,
    /// The messages of the transactions (compacted with trailing zeros)
    #[serde(with = "serde_message_array")]
    #[borsh(
        serialize_with = "serialize_message_array_borsh",
        deserialize_with = "deserialize_message_array_borsh"
    )]
    pub messages: [Element; 1000],
}

impl Default for AggAggPublicInput {
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

mod serde_message_array {
    use element::Element;
    use serde::ser::SerializeSeq;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(messages: &[Element; 1000], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(1000))?;
        for element in messages {
            seq.serialize_element(element)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[Element; 1000], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<Element> = Vec::deserialize(deserializer)?;
        vec.try_into()
            .map_err(|_| serde::de::Error::custom("expected 1000 elements"))
    }
}

fn serialize_message_array_borsh(
    messages: &[Element; 1000],
    writer: &mut impl std::io::Write,
) -> std::io::Result<()> {
    // Write length prefix for Vec compatibility
    (1000u32)
        .to_le_bytes()
        .iter()
        .try_for_each(|b| writer.write_all(&[*b]))?;
    // Write each element
    for msg in messages {
        borsh::BorshSerialize::serialize(msg, writer)?;
    }
    Ok(())
}

#[allow(clippy::large_stack_arrays)]
fn deserialize_message_array_borsh(
    reader: &mut impl std::io::Read,
) -> std::io::Result<[Element; 1000]> {
    // Read length prefix
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes);
    if len != 1000 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "expected 1000 elements",
        ));
    }
    // Read elements
    let mut messages = [Element::ZERO; 1000];
    for msg in &mut messages {
        *msg = borsh::BorshDeserialize::deserialize_reader(reader)?;
    }
    Ok(messages)
}

impl AggAggPublicInput {
    /// Convert the AggAggPublicInput to a AggAggPublicInputBytes
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 * (2 + 1 + 1 + 1 + 1000));

        bytes.extend(self.verification_key_hash[0].to_be_bytes());
        bytes.extend(self.verification_key_hash[1].to_be_bytes());
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
    pub proof: ProofBytes,
    /// The public input for the AggAgg transaction
    pub public_inputs: AggAggPublicInput,
    /// KZG accumulator inputs
    pub kzg: Vec<Element>,
}

impl ToBytes for AggAggProof {
    /// Convert the AggAggProof to a ProofBytes
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
