use contextful::{FromContextful, InternalError};
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// Result alias for the public Bungee domain API.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Public domain errors returned by the Bungee service.
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Unsupported source chain id.
    #[bad_request("unsupported-source-chain-id")]
    #[error("[bungee-interface/client] unsupported source chain id: {chain_id}")]
    UnsupportedSourceChainId {
        /// The unsupported chain id.
        chain_id: u128,
    },

    /// No route available from Bungee.
    #[not_found("bungee-no-route")]
    #[error("[bungee-interface/client] no bungee route available")]
    NoRoute,

    /// Missing identifier for status lookup.
    #[bad_request("bungee-status-missing-identifier")]
    #[error("[bungee-interface/client] missing identifier for bungee status lookup")]
    MissingStatusIdentifier,

    /// Input amount is below the minimum threshold.
    #[bad_request("bungee-input-amount-too-low", severity = "warn")]
    #[error("[bungee-interface/client] input amount too low: ${usd_amount:.2} (minimum $0.10)")]
    InputAmountTooLow {
        /// The actual USD input amount.
        usd_amount: f64,
    },

    /// Output amount is below the minimum threshold.
    #[bad_request("bungee-output-amount-too-low", severity = "warn")]
    #[error("[bungee-interface/client] output amount too low: ${usd_amount:.2} (minimum $0.10)")]
    OutputAmountTooLow {
        /// The actual USD output amount.
        usd_amount: f64,
    },

    /// Internal error.
    #[error("[bungee-interface/client] internal error")]
    #[internal("internal-error", data = "omit")]
    Internal(#[from] InternalError),
}
