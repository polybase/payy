use async_trait::async_trait;
use element::Element;
use unimock::unimock;
use uuid::Uuid;

use crate::{Note, NoteWithPk, RefKind, Result};

/// Notes to be used for a transaction after assignment and database insertion.
#[derive(Debug, Clone)]
pub struct AssignedTxnNotes {
    /// Primary input note, now marked as assigned in storage.
    pub input_note_1: NoteWithPk,
    /// Optional second input note when two notes are needed.
    pub input_note_2: Option<NoteWithPk>,
    /// Primary output note created for the recipient.
    pub output_note_1: NoteWithPk,
    /// Optional change output note returned to the sender.
    pub output_note_2: Option<NoteWithPk>,
}

/// Order preference for selecting notes by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteSelectionOrder {
    /// Select smallest value notes first (ascending order).
    SmallestFirst,
    /// Select largest value notes first (descending order).
    LargestFirst,
}

#[unimock(api = AssignInterfaceMock)]
#[async_trait]
/// Core assignment logic for selecting inputs and creating output notes for spending.
pub trait AssignInterface: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    /// Find notes for the amount, assign inputs/outputs, and execute the transfer.
    async fn assign_and_spend_note_with_amount(
        &self,
        from: Uuid,
        to: Uuid,
        amount: Element,
        note_kind: Element,
        ref_kind: RefKind,
        ref_id: Option<String>,
        selection_order: NoteSelectionOrder,
    ) -> Result<NoteWithPk>;

    #[allow(clippy::too_many_arguments)]
    /// Find a single note from `from` with value >= amount and assign outputs.
    async fn assign_single_owner_note_with_amount(
        &self,
        from: Uuid,
        to: Uuid,
        amount: Element,
        note_kind: Element,
        ref_kind: RefKind,
        ref_id: Option<String>,
        selection_order: NoteSelectionOrder,
    ) -> Result<AssignedTxnNotes>;

    #[allow(clippy::too_many_arguments)]
    /// Use up to two notes (largest first) to cover the amount, with optional strictness.
    async fn assign_multi_owner_note_with_amount_value_desc(
        &self,
        from: Uuid,
        to: Uuid,
        amount: Element,
        note_kind: Element,
        ref_kind: RefKind,
        ref_id: Option<String>,
        strict_amount: bool,
    ) -> Result<AssignedTxnNotes>;

    #[allow(clippy::too_many_arguments)]
    /// Assign the provided input notes and create corresponding output notes.
    async fn assign_given_notes(
        &self,
        input_note_1: Note,
        input_note_2: Option<Note>,
        to: Uuid,
        change_to: Uuid,
        amount: Element,
        ref_kind: RefKind,
        ref_id: Option<String>,
    ) -> Result<AssignedTxnNotes>;
}
