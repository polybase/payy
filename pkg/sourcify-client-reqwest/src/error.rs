use contextful::{Contextful, FromContextful, InternalError};
use sourcify_interface::Error as InterfaceError;

/// Result alias for reqwest Sourcify client internals.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[allow(clippy::needless_pass_by_value)]
fn map_reqwest_error(err: Contextful<reqwest::Error>) -> Error {
    Error::LookupFailed {
        reason: err.to_string(),
    }
}

/// Reqwest Sourcify client errors.
#[derive(Debug, thiserror::Error, FromContextful)]
#[contextful(map_reqwest_error)]
pub enum Error {
    /// Invalid Sourcify endpoint.
    #[error("[sourcify-client-reqwest] invalid endpoint: {endpoint}")]
    InvalidEndpoint {
        /// Configured endpoint.
        endpoint: String,
    },

    /// Sourcify does not have a verified record for the target.
    #[error("[sourcify-client-reqwest] contract is not verified on Sourcify")]
    NotVerified,

    /// Sourcify does not support this chain.
    #[error("[sourcify-client-reqwest] Sourcify does not support chain {chain_id}")]
    ChainUnsupported {
        /// Selected chain id.
        chain_id: u64,
    },

    /// Sourcify lookup failed.
    #[error("[sourcify-client-reqwest] Sourcify lookup failed: {reason}")]
    LookupFailed {
        /// Human-readable failure context.
        reason: String,
    },

    /// Sourcify response exceeded the configured cap.
    #[error("[sourcify-client-reqwest] Sourcify response exceeded {cap_bytes} bytes")]
    ResponseTooLarge {
        /// Command response cap.
        cap_bytes: usize,
    },

    /// Sourcify returned malformed data.
    #[error("[sourcify-client-reqwest] malformed Sourcify response: {reason}")]
    MalformedResponse {
        /// Human-readable parse or validation context.
        reason: String,
    },

    /// Internal error.
    #[error("[sourcify-client-reqwest] internal error")]
    Internal(#[from] InternalError),
}

impl From<Error> for InterfaceError {
    fn from(err: Error) -> Self {
        match err {
            Error::NotVerified => Self::NotVerified,
            Error::ChainUnsupported { chain_id } => Self::ChainUnsupported { chain_id },
            Error::LookupFailed { reason } => Self::LookupFailed { reason },
            Error::ResponseTooLarge { cap_bytes } => Self::ResponseTooLarge { cap_bytes },
            Error::MalformedResponse { reason } => Self::MalformedResponse { reason },
            Error::InvalidEndpoint { endpoint } => Self::LookupFailed {
                reason: format!("invalid Sourcify endpoint: {endpoint}"),
            },
            Error::Internal(internal) => Self::Internal(internal),
        }
    }
}
