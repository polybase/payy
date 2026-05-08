use contextful::{FromContextful, InternalError};
use currency::Currency;
use thiserror::Error;

use crate::TokenIdentifier;

/// Convenience result alias for price cache operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Shared price cache interface errors.
#[derive(Debug, Error, FromContextful)]
pub enum Error {
    #[error("[price-cache-interface] token price not found for {token} in {currency}")]
    PriceNotFound {
        token: TokenIdentifier,
        currency: Currency,
    },

    #[error("[price-cache-interface] internal error")]
    Internal(#[from] InternalError),
}
