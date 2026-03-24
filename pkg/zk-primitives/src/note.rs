use crate::{get_address_for_private_key, hash_private_key_for_psi};
use element::Element;
use noirc_abi::input_parser::InputValue;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

/// A note is used in zk circuits to represent some kind of token (e.g. USDC) on
/// the Payy Network.
///
/// This is used to create notes in the zk-rollup
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// The kind of note
    pub kind: Element,
    /// The contract of note
    pub contract: Element,
    /// The address of the note
    pub address: Element,
    /// The psi adds additional entropy to the note, to ensure uniqueness
    pub psi: Element,
    /// The value of the note (dependent on the token)
    pub value: Element,
}

impl Note {
    /// Create a new note
    #[must_use]
    pub fn new(address: Element, value: Element, note_kind: Element) -> Self {
        Self {
            kind: Element::new(2),
            contract: note_kind,
            address,
            psi: Element::secure_random(thread_rng()),
            value,
        }
    }

    /// Create a new note with custom PSI
    #[must_use]
    pub fn new_with_psi(
        address: Element,
        value: Element,
        psi: Element,
        note_kind: Element,
    ) -> Self {
        Self {
            kind: Element::new(2),
            contract: note_kind,
            address,
            psi,
            value,
        }
    }

    /// New note from ephemeral private key (only use private key once)
    #[must_use]
    pub fn new_from_ephemeral_private_key(
        private_key: Element,
        value: Element,
        note_kind: Element,
    ) -> Self {
        let address = get_address_for_private_key(private_key);
        let psi = hash_private_key_for_psi(private_key);
        Self {
            kind: Element::new(2),
            contract: note_kind,
            address,
            psi,
            value,
        }
    }

    /// Deterministic padding note, because circuits have a fixed note input size,
    /// and so we pad extra notes with zeros
    #[must_use]
    pub fn padding_note() -> Self {
        Note {
            kind: Element::new(2),
            contract: Element::ZERO,
            address: Element::ZERO,
            psi: Element::ZERO,
            value: Element::ZERO,
        }
    }

    /// Check if the note is a padding note
    #[must_use]
    pub fn is_padding_note(&self) -> bool {
        self.contract == Element::ZERO && self.value == Element::ZERO
    }

    /// Commitment of the note, this is stored in the merkle tree and proves the note exists
    // TODO: should we leave some space in here?
    #[must_use]
    pub fn commitment(&self) -> Element {
        if self.value == Element::ZERO {
            Element::ZERO
        } else {
            hash::hash_merge([
                self.kind,
                self.contract,
                self.value,
                self.address,
                self.psi,
                Element::ZERO,
                Element::ZERO,
            ])
        }
    }
}

impl Default for Note {
    fn default() -> Self {
        Self::padding_note()
    }
}

impl From<&Note> for InputValue {
    fn from(note: &Note) -> Self {
        let mut struct_ = BTreeMap::new();

        struct_.insert(
            "address".to_owned(),
            InputValue::Field(note.address.to_base()),
        );
        struct_.insert(
            "kind".to_owned(),
            InputValue::Field(note.contract.to_base()),
        );
        struct_.insert("psi".to_owned(), InputValue::Field(note.psi.to_base()));
        struct_.insert("value".to_owned(), InputValue::Field(note.value.to_base()));

        InputValue::Struct(struct_)
    }
}
