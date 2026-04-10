use contextful::{FromContextful, InternalError};
use currency::Currency;
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// RPC errors for guild token-price lookups.
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// The requested token price is missing from guild's backing price cache.
    #[not_found("prices-token-price-not-found")]
    #[error("[guild-interface/prices] token price not found")]
    TokenPriceNotFound,

    /// Catch-all internal error wrapper.
    #[error("[guild-interface/prices] internal error")]
    Internal(#[from] InternalError),
}

/// Shared query payload for token-price lookups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTokenPriceQuery {
    /// Quote currency for the returned price.
    pub currency: Currency,
}

/// Path payload for symbol-based token-price routes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTokenPriceBySymbolPath {
    /// Asset symbol to resolve against guild's price cache.
    pub symbol: String,
}

/// Path payload for address-based token-price routes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTokenPriceByAddressPath {
    /// Chain/network namespace for the token contract.
    pub network: String,
    /// Contract address for the token.
    pub address: String,
}
