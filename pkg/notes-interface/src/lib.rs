#![warn(clippy::pedantic)]
#![allow(missing_docs)]
#![allow(clippy::ignored_unit_patterns)]

pub mod assign;
pub mod clean_up;
pub mod data;
pub mod error;
mod note_conversion;
pub mod request;
pub mod transfer;
pub mod user;

pub use assign::{AssignInterface, AssignedTxnNotes, NoteSelectionOrder};
pub use clean_up::{CleanUpInterface, CleanUpInterfaceMock};
pub use data::{
    NULL_OWNER_ID, NewNote, Note, NoteStatus, NoteWithPk, PAYY_OWNER_ID, PAYY_OWNER_MIGRATE_ID,
    RefKind, UpdateNote,
};
pub use error::{Error, Result};
pub use request::{NoteRequestInput, NoteRequestKind, RequestInterface};
pub use transfer::TransferInterface;
pub use user::{ListNotesQuery, UserNotesInterface};

use async_trait::async_trait;
use std::sync::Arc;
use unimock::unimock;

#[unimock(api = NotesInterfaceMock)]
#[async_trait]
#[allow(clippy::ignored_unit_patterns)]
/// Main entry point for note operations, exposing sub-interfaces used by guild and notes services.
pub trait NotesInterface: Send + Sync {
    /// User-facing note operations (create, list, and status checks).
    fn user_notes(&self) -> Arc<dyn UserNotesInterface>;
    /// Assignment logic for selecting inputs and preparing outputs before spending.
    fn assign(&self) -> Arc<dyn AssignInterface>;
    /// Transaction execution and proof submission for assigned notes.
    fn transfer(&self) -> Arc<dyn TransferInterface>;
    /// Allocation of notes for external requests (e.g., ramp deposit payouts).
    fn request(&self) -> Arc<dyn RequestInterface>;
    /// Cleanup routines for pending or failed note transactions.
    fn clean_up(&self) -> Arc<dyn CleanUpInterface>;
}
