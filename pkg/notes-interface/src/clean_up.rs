use async_trait::async_trait;
use chrono::Duration;
use element::Element;
use unimock::unimock;

use crate::{NoteStatus, NoteWithPk, Result};

#[unimock(api = CleanUpInterfaceMock)]
#[async_trait]
pub trait CleanUpInterface: Send + Sync {
    /// Spawn background cleanup for timed-out notes using the default timeout.
    fn spawn_clean_up_old_notes_after_timeout(&self);

    /// Spawn background cleanup for timed-out notes immediately.
    fn spawn_clean_up_old_notes_now(&self);

    /// Spawn background cleanup for timed-out notes after a custom delay.
    fn spawn_clean_up_old_notes_after(&self, delay: Duration);

    /// Clean up all timed-out notes (both input and output pending notes)
    async fn clean_up_old_notes(&self) -> Result<()>;

    /// Clean up a note while validating it currently has one of the provided statuses.
    async fn clean_up_note_with_existing_status(
        &self,
        commitment: Element,
        existing_status: &[NoteStatus],
        clear_spend_ref_claim: bool,
        is_output_note: bool,
    ) -> Result<NoteStatus>;

    /// Clean up a single input note, optionally clearing its spend ref claim
    async fn clean_up_input_note(
        &self,
        commitment: Element,
        clear_spend_ref_claim: bool,
    ) -> Result<NoteStatus>;

    /// Clean up a single output note
    async fn clean_up_output_note(&self, commitment: Element) -> Result<NoteStatus>;

    /// Clean up all notes involved in a transaction
    async fn clean_up_transaction_notes(
        &self,
        input_note_1: &NoteWithPk,
        input_note_2: Option<&NoteWithPk>,
        output_note_1: &NoteWithPk,
        output_note_2: Option<&NoteWithPk>,
    ) -> Result<()>;

    /// Spawn cleanup for all transaction notes after the default timeout.
    fn spawn_clean_up_transaction_notes_after_timeout(
        &self,
        input_note_1: &NoteWithPk,
        input_note_2: Option<&NoteWithPk>,
        output_note_1: &NoteWithPk,
        output_note_2: Option<&NoteWithPk>,
    );

    /// Spawn cleanup for all transaction notes after a custom delay.
    fn spawn_clean_up_transaction_notes_after(
        &self,
        input_note_1: &NoteWithPk,
        input_note_2: Option<&NoteWithPk>,
        output_note_1: &NoteWithPk,
        output_note_2: Option<&NoteWithPk>,
        delay: Duration,
    );

    /// Get the current status of a note by querying the node
    async fn get_note_status(
        &self,
        commitment: Element,
        is_output_note: bool,
    ) -> Result<NoteStatus>;
}
