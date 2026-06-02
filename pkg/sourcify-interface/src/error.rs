use contextful::{FromContextful, InternalError};

/// Result alias for Sourcify interface operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Stable Sourcify lookup errors.
#[derive(Debug, thiserror::Error, FromContextful)]
pub enum Error {
    /// Sourcify does not have a verified record for the target.
    #[error("[sourcify-interface] contract is not verified on Sourcify")]
    NotVerified,

    /// Sourcify does not support this chain.
    #[error("[sourcify-interface] Sourcify does not support chain {chain_id}")]
    ChainUnsupported {
        /// Selected chain id.
        chain_id: u64,
    },

    /// Sourcify lookup failed at the transport or service layer.
    #[error("[sourcify-interface] Sourcify lookup failed: {reason}")]
    LookupFailed {
        /// Human-readable failure context.
        reason: String,
    },

    /// Sourcify response exceeded the configured cap.
    #[error("[sourcify-interface] Sourcify response exceeded {cap_bytes} bytes")]
    ResponseTooLarge {
        /// Command response cap.
        cap_bytes: usize,
    },

    /// Sourcify returned an invalid response shape.
    #[error("[sourcify-interface] malformed Sourcify response: {reason}")]
    MalformedResponse {
        /// Human-readable parse or validation context.
        reason: String,
    },

    /// Internal error.
    #[error("[sourcify-interface] internal error")]
    Internal(#[from] InternalError),
}
