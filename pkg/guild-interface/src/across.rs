use contextful::{FromContextful, InternalError};
use contracts::{Address, H256, U256};
use element::Element;
use primitives::serde::{deserialize_hex_0x_prefixed, serialize_hex_0x_prefixed};
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// Across error
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Unsupported source chain id
    #[bad_request("unsupported-source-chain-id")]
    #[error("[guild-interface/across] unsupported source chain id: {chain_id}")]
    UnsupportedSourceChainId {
        /// The unsupported chain id
        chain_id: u128,
    },
    /// Source and destination chain ids must differ
    #[bad_request("same-source-destination-chain-id")]
    #[error(
        "[guild-interface/across] source chain id {source_chain_id} matches destination chain id {destination_chain_id}"
    )]
    SameSourceAndDestinationChainId {
        /// The provided source chain id
        source_chain_id: u128,
        /// The provided destination chain id
        destination_chain_id: u128,
    },
    /// Amount too low error from Across API
    #[bad_request("amount-too-low")]
    #[error("[guild-interface/across] amount too low: {message}")]
    AmountTooLow {
        /// The error message from Across API
        message: String,
    },
    /// Amount too high error from Across API
    #[bad_request("amount-too-high")]
    #[error("[guild-interface/across] amount too high: {message}")]
    AmountTooHigh {
        /// The error message from Across API
        message: String,
    },
    /// Route not enabled error from Across API
    #[bad_request("route-not-enabled")]
    #[error("[guild-interface/across] route not enabled: {message}")]
    RouteNotEnabled {
        /// The error message from Across API
        message: String,
    },
    /// Internal error
    #[error("[guild-interface/across] internal error")]
    Internal(#[from] InternalError),
}

/// Result type for across operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Input for getting an Across quote
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetQuoteInput {
    /// Source chain id
    pub source_chain_id: u128,
    /// Destination chain id
    pub destination_chain_id: u128,
    /// Input token address
    pub input_token: Address,
    /// Output token address
    pub output_token: Address,
    /// Input amount
    pub input_amount: U256,
}

/// Output for getting an Across quote
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetQuoteOutput {
    /// Output amount
    pub output_amount: U256,
    /// Quote timestamp
    pub quote_timestamp: u32,
    /// Fill deadline
    pub fill_deadline: u32,
    /// Exclusivity deadline
    pub exclusivity_deadline: u32,
    /// The relayer for an exclusive quote
    pub exclusive_relayer: Address,
}

/// Input for across deposit
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DepositV3WithAuthorizationInput {
    /// USDC signature
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub usdc_sig: Vec<u8>,
    /// Deposit signature
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub deposit_sig: Vec<u8>,
    /// Valid after
    pub valid_after: U256,
    /// Valid before
    pub valid_before: U256,
    /// Nonce
    pub nonce: Element,
    /// Depositor address
    pub depositor: Address,
    /// Recipient address
    pub recipient: Address,
    /// Input token address
    pub input_token: Address,
    /// Output token address
    pub output_token: Address,
    /// Source chain id
    pub source_chain_id: u128,
    /// Input amount
    pub input_amount: U256,
    /// Output amount
    pub output_amount: U256,
    /// Destination chain id
    pub destination_chain_id: u128,
    /// Exclusive relayer address
    pub exclusive_relayer: Address,
    /// Quote timestamp
    pub quote_timestamp: u32,
    /// Fill deadline
    pub fill_deadline: u32,
    /// Exclusivity deadline
    pub exclusivity_deadline: u32,
    /// Message
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub message: Vec<u8>,
}

/// Output for across deposit
#[derive(Debug, Serialize, Deserialize)]
pub struct DepositV3WithAuthorizationOutput {
    /// Transaction hash
    pub txn: H256,
}
