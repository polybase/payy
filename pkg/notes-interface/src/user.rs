use async_trait::async_trait;
use element::Element;
use serde::{Deserialize, Serialize};
use unimock::unimock;
use uuid::Uuid;

use crate::{Note, NoteStatus, RefKind, Result};

/// Query parameters for list notes
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ListNotesQuery {
    /// Long poll duration
    pub wait: Option<u64>,
    /// Get changes since unix timestamps (in microseconds)
    pub after: Option<u64>,
    /// Limit result count
    pub limit: Option<u16>,
    /// Filter for note status
    pub status: Option<NoteStatus>,
    /// Filter for note kind (token contract).
    pub note_kind: Option<Element>,
}

#[unimock(api = UserNotesInterfaceMock)]
#[async_trait]
/// User-facing note operations backed by storage and node queries.
pub trait UserNotesInterface: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    /// Create a note for a wallet from external deposit data.
    async fn create_user_notes(
        &self,
        wallet_id: Uuid,
        private_key: Element,
        psi: Element,
        value: Element,
        note_kind: Element,
        ref_kind: RefKind,
        ref_kind_id: Option<String>,
    ) -> Result<Note>;

    /// List notes owned by a wallet, filtered by query parameters.
    async fn list_notes_by_wallet_id(
        &self,
        wallet_id: Uuid,
        query: ListNotesQuery,
    ) -> Result<Vec<Note>>;

    /// List notes owned by a wallet, filtered by `ref_id` (`spend_ref_id` OR `received_ref_id`).
    async fn list_notes_by_wallet_id_and_ref_id(
        &self,
        wallet_id: Uuid,
        ref_id: &str,
    ) -> Result<Vec<Note>>;

    /// List notes owned by a wallet, filtered by multiple statuses.
    async fn list_notes_by_wallet_id_and_status(
        &self,
        wallet_id: Uuid,
        status: Vec<NoteStatus>,
    ) -> Result<Vec<Note>>;

    /// Partition notes into (unspent, spent) by checking on-chain state.
    async fn check_existing_notes_spent<'a>(
        &self,
        notes: Vec<&'a Note>,
    ) -> Result<(Vec<&'a Note>, Vec<&'a Note>)>;
}
