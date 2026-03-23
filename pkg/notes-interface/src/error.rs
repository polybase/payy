use contextful::{FromContextful, InternalError};
use rpc::{
    HTTPErrorConversion,
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error, HTTPErrorConversion, FromContextful)]
/// Errors returned by notes-interface operations, mapped to RPC-friendly codes.
pub enum Error {
    #[error("[notes-interface] note was claimed by another transaction")]
    #[aborted("note-was-claimed-by-another-transaction")]
    NoteWasClaimedByAnotherTransaction,

    #[error("[notes-interface] no notes available to claim")]
    #[failed_precondition("no-notes-available-to-claim")]
    NoNotesAvailableToClaim,

    #[error("[notes-interface] note does not exist")]
    #[failed_precondition("note-not-exist")]
    NoteNotExist,

    #[error("[notes-interface] note is already spent")]
    #[failed_precondition("note-already-spent")]
    NoteAlreadySpent,

    #[error("[notes-interface] note already exists")]
    #[already_exists("note-already-exists")]
    NoteAlreadyExists,

    #[error("[notes-interface] input notes kind mismatch")]
    #[failed_precondition("input-notes-kind-mismatch")]
    InputNotesKindMismatch,

    #[error("[notes-interface] transaction exceeded deadline")]
    #[internal("txn-exceeded-deadline")]
    TxnExceededDeadline,

    #[error("[notes-interface] unable to generate utxo proof")]
    #[internal("unable-to-generate-utxo-proof")]
    UnableToGenerateUtxoProof,

    #[error("[notes-interface] note request transaction not found")]
    #[not_found("note-request-transaction-not-found")]
    NoteRequestTransactionNotFound,

    #[error("[notes-interface] invalid note request transaction")]
    #[bad_request("note-request-invalid-transaction")]
    InvalidNoteRequestTransaction,

    #[error("[notes-interface] invalid authentication")]
    #[unauthenticated("invalid-auth")]
    InvalidAuth,

    #[error("[notes-interface] internal error")]
    #[internal("internal-error", data = "omit")]
    Internal(#[from] InternalError),
}
