use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use unimock::unimock;
use uuid::Uuid;

use crate::{Note, Result};

/// Kind of note request (reason for requesting a note)
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NoteRequestKind {
    /// Transaction to be paid out
    RampDeposit,
}

/// Kind of note request (reason for requesting a note)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NoteRequestInput {
    /// ID of the note request kind
    pub id: String,
    /// Kind of note request
    pub kind: NoteRequestKind,
}

#[unimock(api = RequestInterfaceMock)]
#[async_trait]
/// Handles allocation of notes for external request flows (e.g., ramp deposits).
pub trait RequestInterface: Send + Sync {
    /// Allocate or return an existing note for the requested purpose.
    async fn request_note(
        &self,
        wallet_id: Uuid,
        ref_id: String,
        ref_kind: NoteRequestKind,
    ) -> Result<Note>;
}

#[cfg(test)]
mod tests;
