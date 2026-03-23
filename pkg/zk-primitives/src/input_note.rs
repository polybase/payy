use crate::note::Note;
use crate::{bridged_polygon_usdc_note_kind, get_address_for_private_key};
use element::Element;
use parse_link::{NoteURLPayload, decode_activity_url_payload};
use serde::{Deserialize, Serialize};

/// InputNote is a Note that belongs to the current user, i.e. they have the
/// spending sercret key and can therefore use it as an input, "spending" the note. Extra
/// constraints need to be applied to input notes to ensure they are valid.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InputNote {
    /// The note to spend
    pub note: Note,
    /// Secret key for the address, required to spend a note
    pub secret_key: Element,
}

impl InputNote {
    /// Create a new input note
    #[must_use]
    pub fn new(note: Note, secret_key: Element) -> Self {
        Self { note, secret_key }
    }

    /// Create a new padding note
    #[must_use]
    pub fn padding_note() -> Self {
        Self {
            secret_key: Element::ZERO,
            note: Note::padding_note(),
        }
    }

    /// Generates a new note with given value, for an ephemeral private key, the private key
    /// must only be used once
    #[must_use]
    pub fn new_from_ephemeral_private_key(
        private_key: Element,
        value: Element,
        note_kind: Element,
    ) -> Self {
        Self {
            note: Note::new_from_ephemeral_private_key(private_key, value, note_kind),
            secret_key: private_key,
        }
    }

    /// Generates an InputNote from a link string e.g. /s#A0F3...
    #[must_use]
    pub fn new_from_link(link: &str) -> Self {
        InputNote::from(&decode_activity_url_payload(link))
    }

    /// Generates a Payy link from the Note + Private Key
    #[must_use]
    pub fn generate_link(&self) -> String {
        self.to_note_url_payload().encode_activity_url_payload()
    }

    /// Construct the canonical [`NoteURLPayload`] representing this note.
    #[must_use]
    pub fn to_note_url_payload(&self) -> NoteURLPayload {
        NoteURLPayload {
            version: 2,
            private_key: self.secret_key,
            psi: None,
            value: self.note.value,
            note_kind: (self.note.contract != bridged_polygon_usdc_note_kind())
                .then_some(self.note.contract),
            referral_code: String::new(),
        }
    }
}

impl From<&NoteURLPayload> for InputNote {
    fn from(value: &NoteURLPayload) -> Self {
        let psi = value.psi();

        Self {
            secret_key: value.private_key,
            note: Note {
                kind: Element::new(2),
                contract: value.note_kind(),
                address: get_address_for_private_key(value.private_key),
                psi,
                value: value.value,
            },
        }
    }
}

#[cfg(test)]
mod tests;
