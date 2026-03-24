use contextful::{FromContextful, InternalError};
use rpc::error::{ErrorOutput, HTTPError, TryFromHTTPError};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

use crate::{across, mint};

/// RPC errors for guild
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Mint error
    // No extra context needed; nested error already includes context.
    #[delegate]
    #[error("[guild-interface/error] mint error")]
    Mint(#[from] mint::Error),

    /// Across error
    // No extra context needed; nested error already includes context.
    #[delegate]
    #[error("[guild-interface/error] across error")]
    Across(#[from] across::Error),

    /// Internal error
    #[error("[guild-interface/error] internal error")]
    Internal(#[from] InternalError),
}

/// Result type for guild interface errors.
pub type Result<T> = std::result::Result<T, Error>;
