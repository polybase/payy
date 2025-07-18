use std::fmt::Debug;

use crate::impl_serde_for_element_array;
use crate::traits::ToBytes;
use crate::{bytes_to_elements, InputNote, Note};
use borsh::{BorshDeserialize, BorshSerialize};
use element::{Base, Element};
use hash::hash_merge;
use primitives::serde::{deserialize_base64, serialize_base64};
use serde::{Deserialize, Serialize};

/// Number of public input fields for utxo proof
pub const UTXO_PUBLIC_INPUTS_COUNT: usize = 10;
/// Number of fields in the proof
pub const UTXO_PROOF_SIZE: usize = 456;

/// Utxo is the data required to prove a utxo transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    /// The kind of transaction
    pub kind: UtxoKind,
    /// The input notes (being spent)
    pub input_notes: [InputNote; 2],
    /// The output notes (being created)
    pub output_notes: [Note; 2],
    /// The burn address
    pub burn_address: Option<Element>,
}

impl Utxo {
    /// Creates a new utxo txn
    #[must_use]
    pub fn new(
        kind: UtxoKind,
        input_notes: [InputNote; 2],
        output_notes: [Note; 2],
        burn_address: Option<Element>,
    ) -> Self {
        Self {
            kind,
            input_notes,
            output_notes,
            burn_address,
        }
    }

    /// Create a new send transaction
    #[must_use]
    pub fn new_send(input_notes: [InputNote; 2], output_notes: [Note; 2]) -> Self {
        Self {
            kind: UtxoKind::Send,
            input_notes,
            output_notes,
            burn_address: None,
        }
    }

    /// Create a new burn transaction
    #[must_use]
    pub fn new_burn(input_notes: [InputNote; 2], evm_address: Element) -> Self {
        Self {
            kind: UtxoKind::Burn,
            input_notes,
            output_notes: [Note::padding_note(), Note::padding_note()],
            burn_address: Some(evm_address),
        }
    }

    /// Create a new mint transaction
    #[must_use]
    pub fn new_mint(output_notes: [Note; 2]) -> Self {
        Self {
            kind: UtxoKind::Mint,
            burn_address: None,
            input_notes: [InputNote::padding_note(), InputNote::padding_note()],
            output_notes,
        }
    }

    /// Get the leaf elements for the Utxo transaction, these will be inserted
    /// or removed from the tree
    #[must_use]
    pub fn leaf_elements(&self) -> [Element; 4] {
        [
            self.input_notes[0].note.commitment(),
            self.input_notes[1].note.commitment(),
            self.output_notes[0].commitment(),
            self.output_notes[1].commitment(),
        ]
    }

    /// Get the messages for the Utxo transaction
    #[must_use]
    pub fn messages(&self) -> [Element; 6] {
        match self.kind {
            UtxoKind::Send => [
                Element::new(1),
                Element::ZERO,
                Element::ZERO,
                Element::ZERO,
                Element::ZERO,
                Element::ZERO,
            ],
            UtxoKind::Mint => [
                Element::new(2),
                self.output_notes[0].kind,
                self.output_value() - self.input_value(),
                self.mint_hash(),
                Element::ZERO,
                Element::ZERO,
            ],
            UtxoKind::Burn => [
                Element::new(3),
                self.input_notes[0].note.kind,
                self.input_value() - self.output_value(),
                self.burn_hash(),
                self.burn_address.unwrap(),
                Element::ZERO,
            ],
            UtxoKind::Null => [
                Element::ZERO,
                Element::ZERO,
                Element::ZERO,
                Element::ZERO,
                Element::ZERO,
                Element::ZERO,
            ],
        }
    }

    /// Get the mint hash
    #[must_use]
    pub fn mint_hash(&self) -> Element {
        hash_merge([self.output_notes[0].psi, self.output_notes[1].psi])
    }

    /// Get the burn hash
    #[must_use]
    pub fn burn_hash(&self) -> Element {
        self.input_notes[0].note.commitment()
    }

    /// Get the input value for the Utxo transaction
    #[must_use]
    pub fn input_value(&self) -> Element {
        self.input_notes[0].note.value + self.input_notes[1].note.value
    }

    /// Get the output value for the Utxo transaction
    #[must_use]
    pub fn output_value(&self) -> Element {
        self.output_notes[0].value + self.output_notes[1].value
    }

    /// Get the public inputs for the Utxo transaction
    #[must_use]
    pub fn public_inputs(&self) -> UtxoPublicInput {
        UtxoPublicInput {
            input_commitments: [
                self.input_notes[0].note.commitment(),
                self.input_notes[1].note.commitment(),
            ],
            output_commitments: [
                self.output_notes[0].commitment(),
                self.output_notes[1].commitment(),
            ],
            messages: self.messages(),
        }
    }

    /// Create a new padding Utxo
    #[must_use]
    pub fn new_padding() -> Self {
        Self {
            kind: UtxoKind::Null,
            input_notes: [InputNote::padding_note(), InputNote::padding_note()],
            output_notes: [Note::padding_note(), Note::padding_note()],
            burn_address: None,
        }
    }

    /// Returns true if the Utxo is a padding Utxo
    #[must_use]
    pub fn is_padding(&self) -> bool {
        self.kind == UtxoKind::Null
    }
}

/// The kind of Utxo transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UtxoKind {
    /// A null transaction (padding UTXO)
    Null,
    /// Send a note (of the same token) to another address
    Send,
    /// A mint transaction (minting on Payy Network)
    Mint,
    /// A burn transaction (burning on Payy Network)
    Burn,
}

impl UtxoKind {
    /// Convert the UtxoKind to an Element
    #[must_use]
    pub fn to_element(&self) -> Element {
        Element::from(u8::from(*self))
    }
}

impl From<u8> for UtxoKind {
    fn from(value: u8) -> Self {
        match value {
            1 => UtxoKind::Send,
            2 => UtxoKind::Mint,
            3 => UtxoKind::Burn,
            _ => UtxoKind::Null,
        }
    }
}

impl From<UtxoKind> for u8 {
    fn from(value: UtxoKind) -> Self {
        match value {
            UtxoKind::Send => 1,
            UtxoKind::Mint => 2,
            UtxoKind::Burn => 3,
            UtxoKind::Null => 0,
        }
    }
}

impl From<Element> for UtxoKind {
    fn from(value: Element) -> Self {
        value.to_u256().as_u8().into()
    }
}

/// The proof for a Utxo transaction (does NOT include public inputs)
#[derive(Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct UtxoProofBytes(
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    pub Vec<u8>,
);

impl Debug for UtxoProofBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x")?;
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Default for UtxoProofBytes {
    fn default() -> Self {
        Self(vec![0; 32 * UTXO_PROOF_SIZE])
    }
}

impl UtxoProofBytes {
    /// Convert the UtxoProofBytes to a UtxoProof
    #[must_use]
    pub fn to_fields(&self) -> Vec<Element> {
        bytes_to_elements(&self.0)
    }
}

/// The public input for a Utxo transaction
#[derive(
    Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct UtxoPublicInput {
    /// The input commitments
    pub input_commitments: [Element; 2],
    /// The output commitments
    pub output_commitments: [Element; 2],
    /// The message of the transaction
    pub messages: [Element; 6],
}

impl UtxoPublicInput {
    /// Convert the UtxoPublicInput to a UtxoPublicInputBytes
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        [
            self.input_commitments[0].to_be_bytes(),
            self.input_commitments[1].to_be_bytes(),
            self.output_commitments[0].to_be_bytes(),
            self.output_commitments[1].to_be_bytes(),
            self.messages[0].to_be_bytes(),
            self.messages[1].to_be_bytes(),
            self.messages[2].to_be_bytes(),
            self.messages[3].to_be_bytes(),
            self.messages[4].to_be_bytes(),
            self.messages[5].to_be_bytes(),
        ]
        .concat()
    }

    /// Fields
    #[must_use]
    pub fn fields(&self) -> [Element; 10] {
        [
            self.input_commitments[0],
            self.input_commitments[1],
            self.output_commitments[0],
            self.output_commitments[1],
            self.messages[0],
            self.messages[1],
            self.messages[2],
            self.messages[3],
            self.messages[4],
            self.messages[5],
        ]
    }

    /// Get the commitments for the UtxoPublicInput
    #[must_use]
    pub fn commitments(&self) -> [Element; 4] {
        [
            self.input_commitments[0],
            self.input_commitments[1],
            self.output_commitments[0],
            self.output_commitments[1],
        ]
    }

    /// Get the commit hash for the Utxo proof
    #[must_use]
    pub fn commit_hash(&self) -> Element {
        hash_merge(self.commitments())
    }

    /// Hash the UtxoProof
    #[must_use]
    pub fn hash(&self) -> Element {
        hash::hash_merge(self.fields())
    }

    /// Get the kind of the UtxoProof
    #[must_use]
    pub fn kind(&self) -> UtxoKind {
        // Get the kind from the first byte of the first message
        let msg_bytes = self.messages[0].to_be_bytes();
        let kind = msg_bytes[31];
        UtxoKind::from(kind)
    }

    /// Get the kind messages associated with the kind
    #[must_use]
    pub fn kind_messages(&self) -> UtxoKindMessages {
        match self.kind() {
            UtxoKind::Null | UtxoKind::Send => UtxoKindMessages::None,
            UtxoKind::Mint => UtxoKindMessages::Mint(UtxoKindMintMessages {
                note_kind: self.messages[1],
                value: self.messages[2],
                mint_hash: self.messages[3],
            }),
            UtxoKind::Burn => UtxoKindMessages::Burn(UtxoKindBurnMessages {
                note_kind: self.messages[1],
                value: self.messages[2],
                burn_hash: self.messages[3],
                burn_address: self.messages[4],
            }),
        }
    }

    /// Gets the hash of the mint/burn, otherwise None
    #[must_use]
    pub fn mint_burn_hash(&self) -> Option<Element> {
        match self.kind_messages() {
            UtxoKindMessages::Mint(mint) => Some(mint.mint_hash),
            UtxoKindMessages::Burn(burn) => Some(burn.burn_hash),
            UtxoKindMessages::None => None,
        }
    }
}

/// Kind messages for each Utxo kind
#[derive(Debug, Clone)]
pub enum UtxoKindMessages {
    /// No relevant messages
    None,
    /// Burn messages
    Burn(UtxoKindBurnMessages),
    /// Mint messages
    Mint(UtxoKindMintMessages),
}

/// Structured messages for burn
#[derive(Debug, Clone)]
pub struct UtxoKindBurnMessages {
    /// Kind of note (USDC, etc)
    pub note_kind: Element,
    /// Value of the note
    pub value: Element,
    /// Hash of the burn
    pub burn_hash: Element,
    /// EVM Address to send funds to
    pub burn_address: Element,
}

/// Structured messages for mint
#[derive(Debug, Clone)]
pub struct UtxoKindMintMessages {
    /// Kind of note (USDC, etc)
    pub note_kind: Element,
    /// Value of the note
    pub value: Element,
    /// Hash of the mint
    pub mint_hash: Element,
}

/// Proof as field elements (instead of bytes)
#[derive(Debug, Clone)]
pub struct UtxoProofFields(pub [Element; 93]);
impl_serde_for_element_array!(UtxoProofFields, 93);

impl From<UtxoProofFields> for [Base; 93] {
    fn from(value: UtxoProofFields) -> Self {
        value.0.map(|e| e.to_base())
    }
}

impl From<[Base; 93]> for UtxoProofFields {
    fn from(elements: [Base; 93]) -> Self {
        UtxoProofFields(elements.map(Element::from_base))
    }
}

/// The output proof for a Utxo transaction
#[derive(Default, Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct UtxoProof {
    /// The proof for the Utxo transaction
    pub proof: UtxoProofBytes,
    /// The public input for the Utxo transaction
    pub public_inputs: UtxoPublicInput,
}

impl PartialEq for UtxoProof {
    fn eq(&self, other: &Self) -> bool {
        self.public_inputs == other.public_inputs
    }
}

impl Eq for UtxoProof {}

impl ToBytes for UtxoProof {
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

impl UtxoProof {
    /// Hash the UtxoProof, can be used to uniquely identify the UtxoProof
    #[must_use]
    pub fn hash(&self) -> Element {
        self.public_inputs.hash()
    }

    /// Get the mint/burn hash of the Utxo transaction
    #[must_use]
    pub fn kind_messages(&self) -> UtxoKindMessages {
        self.public_inputs.kind_messages()
    }

    /// Get the kind of the Utxo transaction
    #[must_use]
    pub fn kind(&self) -> UtxoKind {
        self.public_inputs.kind()
    }

    /// Gets the hash of the mint/burn, otherwise None
    #[must_use]
    pub fn mint_burn_hash(&self) -> Option<Element> {
        self.public_inputs.mint_burn_hash()
    }

    /// Returns true if the Utxo is a padding Utxo
    #[must_use]
    pub fn is_padding(&self) -> bool {
        self.public_inputs.kind() == UtxoKind::Null
    }
}
