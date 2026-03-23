// lint-long-file-override allow-max-lines=300
#[cfg(feature = "diesel")]
use database::schema::notes;
#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    prelude::*,
    serialize::{IsNull, ToSql},
    sql_types::Text,
};
use element::Element;
use hash::hash_merge;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
#[cfg(feature = "diesel")]
use std::io::Write;
use strum::{Display, EnumString};
use uuid::Uuid;
/// Owner ID used for unassigned/orphan notes (e.g., ramp deposits before assignment).
pub const NULL_OWNER_ID: Uuid = Uuid::nil();
/// System-owned notes pool for platform operations.
pub const PAYY_OWNER_ID: Uuid = Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
/// Migration-specific owner identifier for legacy note moves.
pub const PAYY_OWNER_MIGRATE_ID: Uuid =
    Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
#[derive(Debug, Clone, Serialize, Deserialize)]
/// A note paired with its private key for signing/proving spend transactions.
pub struct NoteWithPk {
    pub private_key: Element,
    pub note: zk_primitives::Note,
}

impl NoteWithPk {
    #[must_use]
    /// Create a note with a random private key and address derived from that key.
    pub fn new_with_value(value: Element, note_kind: Element) -> Self {
        let private_key = Element::secure_random(thread_rng());
        let address = hash_merge([private_key, Element::ZERO]);
        let note = zk_primitives::Note::new(address, value, note_kind);
        Self { private_key, note }
    }

    #[must_use]
    /// Create a note with a random ephemeral private key for output notes.
    pub fn new_with_value_ephemeral_private_key(value: Element, note_kind: Element) -> Self {
        let private_key = Element::secure_random(thread_rng());
        let note =
            zk_primitives::Note::new_from_ephemeral_private_key(private_key, value, note_kind);
        Self { private_key, note }
    }
}
#[derive(
    Debug,
    Display,
    Default,
    Serialize,
    Deserialize,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    EnumString,
)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// Lifecycle status for a note stored in the database.
pub enum NoteStatus {
    #[default]
    /// Ready for spending by the owner
    Unspent,
    /// Note has been allocated as an input note for a txn
    TxnInputAssigned,
    /// New possible note that will be the output of a txn
    TxnOutputPending,
    /// Note has been spent, and can no longer be used
    Spent,
    /// Note was the output of a txn, but the txn failed
    Dropped,
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for NoteStatus {
    fn to_sql(&self, out: &mut diesel::serialize::Output<Pg>) -> diesel::serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for NoteStatus {
    fn from_sql(bytes: diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized status".into())
    }
}

#[derive(
    Debug,
    Display,
    Serialize,
    Deserialize,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    EnumString,
)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// Reason a note was created or spent, used to relate notes to external flows.
pub enum RefKind {
    /// Notes associated with NFT-related flows.
    Nft,
    /// Notes created/spent as part of a ramp transaction.
    RampTransaction,
    /// Notes representing change output from a transaction.
    TxnChange,
    /// Notes created by direct user actions.
    User,
    /// Notes created during legacy migration flows.
    MigrateV0,
    /// Notes used to merge system-owned pools.
    PayyOwnerMerge,
    /// Notes created for deposit link flows.
    DepositLink,
    /// Notes created for withdraw link flows.
    WithdrawLink,
    /// Notes representing EVM burn operations.
    BurnEvm,
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for RefKind {
    fn to_sql(&self, out: &mut diesel::serialize::Output<Pg>) -> diesel::serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for RefKind {
    fn from_sql(bytes: diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized ref kind".into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Queryable, Selectable, Identifiable))]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = notes))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
/// Full database representation of a note and its metadata.
pub struct Note {
    /// Primary identifier for the note row.
    pub id: Uuid,
    /// Note kind/contract identifier.
    pub kind: Element,
    /// Commitment hash for the note.
    pub commitment: Element,
    /// Address derived from the note's private key.
    pub address: Element,
    /// Private key stored for spend authorization.
    pub private_key: Element,
    /// Note randomness/secret.
    pub psi: Element,
    /// Note value.
    pub value: Element,
    /// Current lifecycle status.
    pub status: NoteStatus,
    /// First parent note (input) identifier when created via transfer.
    pub parent_1_id: Option<Uuid>,
    /// Optional second parent note identifier.
    pub parent_2_id: Option<Uuid>,
    /// Reference kind for the spending action.
    pub spend_ref_kind: Option<RefKind>,
    /// External reference ID for the spending action.
    pub spend_ref_id: Option<String>,
    /// Reference kind for the receiving action.
    pub received_ref_kind: Option<RefKind>,
    /// External reference ID for the receiving action.
    pub received_ref_id: Option<String>,
    /// Timestamp when the note was spent.
    pub spent_at: Option<chrono::NaiveDateTime>,
    /// Current owner wallet ID.
    pub owner_id: Uuid,
    /// Timestamp when the note was created.
    pub added_at: chrono::NaiveDateTime,
    /// Timestamp when the note was last updated.
    pub updated_at: chrono::NaiveDateTime,
    #[cfg_attr(feature = "diesel", diesel(deserialize_as = i16))]
    /// Schema/version marker for rollup compatibility.
    pub version: u16,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = notes))]
/// Insert struct for creating new note rows.
pub struct NewNote {
    pub id: Option<Uuid>,
    pub kind: Element,
    pub commitment: Element,
    pub address: Element,
    pub private_key: Element,
    pub psi: Element,
    pub value: Element,
    pub status: NoteStatus,
    pub parent_1_id: Option<Uuid>,
    pub parent_2_id: Option<Uuid>,
    pub received_ref_kind: Option<RefKind>,
    pub received_ref_id: Option<String>,
    pub owner_id: Uuid,
    pub version: i16,
}

impl NewNote {
    #[must_use]
    /// Build a pending output note from a `NoteWithPk` and parent metadata.
    pub fn new_with_note(
        note: &NoteWithPk,
        parent_1_id: Uuid,
        parent_2_id: Option<Uuid>,
        owner_id: Uuid,
        received_ref_kind: Option<RefKind>,
        received_ref_id: Option<String>,
    ) -> Self {
        Self {
            id: Some(Uuid::new_v4()),
            kind: note.note.contract,
            commitment: note.note.commitment(),
            address: note.note.address,
            private_key: note.private_key,
            psi: note.note.psi,
            value: note.note.value,
            status: NoteStatus::TxnOutputPending,
            parent_1_id: Some(parent_1_id),
            parent_2_id,
            owner_id,
            received_ref_kind,
            received_ref_id,
            // New rollup is 1
            version: 1,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = notes))]
/// Changeset for updating note status and spend metadata.
pub struct UpdateNote {
    pub status: NoteStatus,
    pub spend_ref_kind: Option<RefKind>,
    pub spend_ref_id: Option<String>,
    pub spent_at: Option<chrono::NaiveDateTime>,
}
