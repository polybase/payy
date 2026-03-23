use async_trait::async_trait;
use unimock::unimock;

use crate::{AssignedTxnNotes, NoteWithPk, Result};

#[unimock(api = TransferInterfaceMock)]
#[async_trait]
/// Executes ZK note transactions and submits them to the node.
pub trait TransferInterface: Send + Sync {
    /// Generate a UTXO proof, submit the transaction, and clean up on failure.
    async fn spend_assigned_note(&self, assigned_notes: AssignedTxnNotes) -> Result<()>;

    /// Low-level transaction submission without cleanup orchestration.
    async fn transfer_notes_txn(
        &self,
        input_note_1: NoteWithPk,
        input_note_2: Option<NoteWithPk>,
        output_note_1: NoteWithPk,
        output_note_2: Option<NoteWithPk>,
    ) -> Result<()>;
}
