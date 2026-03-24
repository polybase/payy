use contextful::{FromContextful, InternalError};
pub use notes_interface::{NoteRequestInput, NoteRequestKind};
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// RPC errors for guild
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Transaction not found for note request
    #[not_found("note-request-transaction-not-found")]
    #[error("[guild-interface/notes/request] transaction not found")]
    TransactionNotFound,

    /// Invalid ramp transaction for notes request
    #[bad_request("note-request-invalid-transaction")]
    #[error("[guild-interface/notes/request] invalid transaction for note request")]
    InvalidTransaction,

    /// Internal error
    #[error("[guild-interface/notes/request] internal error")]
    Internal(#[from] InternalError),
}

/// Result type for note request operations.
pub type Result<T> = std::result::Result<T, Error>;
