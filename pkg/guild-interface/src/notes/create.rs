use contextful::{FromContextful, InternalError};
use element::Element;
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};
use zk_primitives::bridged_polygon_usdc_note_kind;

/// RPC errors for guild
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Note already exists on the server
    #[already_exists("note-already-exists")]
    #[error("[guild-interface/notes/create] note already exists")]
    NoteAlreadyExists,

    /// Note has already been spent so cannot be added
    #[already_exists("note-already-spent")]
    #[error("[guild-interface/notes/create] note is already spent")]
    NoteAlreadySpent,

    /// Internal error
    #[error("[guild-interface/notes/create] internal error")]
    Internal(#[from] InternalError),
}

/// Result type for note creation operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Input for creating a new note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNoteInput {
    /// Private key of the note
    pub private_key: Element,
    /// Randomness (psi) of the note
    pub psi: Element,
    /// Value of the note
    pub value: Element,
    /// Note kind / contract. Defaults to USDC for older clients.
    #[serde(default = "bridged_polygon_usdc_note_kind")]
    pub note_kind: Element,
}
