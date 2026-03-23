use element::Element;
pub use notes_interface::{NoteStatus, RefKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Guild note
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Note {
    /// ID of the note
    pub id: Uuid,
    /// Kind of note
    pub kind: Element,
    /// Commitment of note
    pub commitment: Element,
    /// Address of the private key
    pub address: Element,
    /// Private key
    pub private_key: Element,
    /// Randomness
    pub psi: Element,
    /// Value of the note
    pub value: Element,
    /// Status of the note
    pub status: NoteStatus,
    /// Reason kind for spend
    pub spend_ref_kind: Option<RefKind>,
    /// Reason id for spend
    pub spend_ref_id: Option<String>,
    /// Reason kind for note received/created
    pub received_ref_kind: Option<RefKind>,
    /// Reason id for note received/created
    pub received_ref_id: Option<String>,
    /// Owner ID of the note (as all private keys are ephemeral)
    pub owner_id: Uuid,
    /// Date/time note added
    pub added_at: chrono::NaiveDateTime,
    /// Date/time updated (for long polling/since)
    pub updated_at: chrono::NaiveDateTime,
}

impl From<notes_interface::Note> for Note {
    fn from(note: notes_interface::Note) -> Note {
        Note {
            id: note.id,
            kind: note.kind,
            commitment: note.commitment,
            address: note.address,
            private_key: note.private_key,
            psi: note.psi,
            value: note.value,
            status: note.status,
            spend_ref_kind: note.spend_ref_kind,
            spend_ref_id: note.spend_ref_id,
            received_ref_kind: note.received_ref_kind,
            received_ref_id: note.received_ref_id,
            owner_id: note.owner_id,
            added_at: note.added_at,
            updated_at: note.updated_at,
        }
    }
}

impl From<Note> for zk_primitives::Note {
    fn from(note: Note) -> zk_primitives::Note {
        zk_primitives::Note {
            kind: Element::new(2),
            contract: note.kind,
            address: note.address,
            psi: note.psi,
            value: note.value,
        }
    }
}

impl From<Note> for zk_primitives::InputNote {
    fn from(note: Note) -> zk_primitives::InputNote {
        let secret_key = note.private_key;
        zk_primitives::InputNote {
            note: zk_primitives::Note::from(note),
            secret_key,
        }
    }
}
