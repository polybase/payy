use element::Element;
use serde::{Deserialize, Serialize};

pub use notes_interface::NoteWithPk;

/// Request body input for migrate notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationNotesRequest {
    /// Notes to be migrated
    pub notes: Vec<MigrationNote>,
}

/// Response for migrate notes endpoint
pub type MigrationNotesResponse = Vec<NoteWithPk>;

/// Old rollup note to be migrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationNote {
    /// Value of the note
    pub value: Element,
    /// Address of the note
    pub address: Element,
    /// Randomness of note
    pub psi: Element,
    /// Private key of note
    pub private_key: Element,
}
