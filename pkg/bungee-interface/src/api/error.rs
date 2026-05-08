use contextful::{FromContextful, InternalError};

/// Transport-only errors returned by the upstream API calque.
#[derive(Debug, thiserror::Error, FromContextful)]
pub enum TransportError {
    /// Request timed out.
    #[error("[bungee-interface/api] request timed out: {reason}")]
    Timeout {
        /// Human-readable timeout reason from the transport.
        reason: String,
    },

    /// Internal transport error.
    #[error("[bungee-interface/api] internal error")]
    Internal(#[from] InternalError),
}
