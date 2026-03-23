use contextful::{FromContextful, InternalError};
use element::Element;
use primitives::block_height::BlockHeight;
use primitives::serde::Base64Bytes;
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

pub use data::registry_note::RegistryNote;

/// Query parameters for registry notes
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ListRegistryNotesQuery {
    /// Long poll duration
    pub wait: Option<u64>,
    /// Get added since unix timestamps (in microseconds)
    pub after: Option<u64>,
    /// Limit result count
    pub limit: Option<u16>,
}

/// Create a new note entry into the registry
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateRegistryNoteInput {
    /// Block the note was spent in
    pub block: BlockHeight,
    /// Public key of the user funds are being sent to
    pub public_key: Element,
    /// Symmetric key, encrypted by public key
    pub encrypted_key: Base64Bytes,
    /// Note data, encrypted by symmetric key
    pub encrypted_note: Base64Bytes,
}

impl CreateRegistryNoteInput {
    /// Validate the entry
    pub fn validate(&self) -> Result<()> {
        if self.encrypted_key.len() > 1024 {
            return Err(Error::DataTooLong);
        }

        if self.encrypted_note.len() > 1024 {
            return Err(Error::DataTooLong);
        }

        Ok(())
    }
}

/// Registry error
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Data provided is too long
    #[bad_request("data-too-long")]
    #[error("[guild-interface/registry] data is too long")]
    DataTooLong,

    /// Internal error
    #[error("[guild-interface/registry] internal error")]
    Internal(#[from] InternalError),
}

/// Result type for registry operations.
pub type Result<T> = std::result::Result<T, Error>;
