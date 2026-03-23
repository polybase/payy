//! Bungee interface request/response types for server <-> clients.

// lint-long-file-override allow-max-lines=500
use contracts::{Address, U256};
use primitives::serde::{deserialize_hex_0x_prefixed, serialize_hex_0x_prefixed};
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Bungee error
#[derive(Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize)]
pub enum Error {
    /// Unsupported source chain id
    #[bad_request("unsupported-source-chain-id")]
    #[error("unsupported source chain id: {chain_id}")]
    UnsupportedSourceChainId {
        /// The unsupported chain id
        chain_id: u128,
    },

    /// No route available from Bungee (autoRoute missing/null)
    #[not_found("bungee-no-route")]
    #[error("no bungee route available for the requested swap/bridge")]
    NoRoute,

    /// Missing identifier for status lookup
    #[bad_request("bungee-status-missing-identifier")]
    #[error("missing identifier for bungee status lookup")]
    MissingStatusIdentifier,

    /// Input amount is below minimum threshold ($0.10 USD)
    #[bad_request("bungee-input-amount-too-low", severity = "warn")]
    #[error("[bungee] bungee quote input amount too low: ${usd_amount:.2} (minimum $0.10)")]
    InputAmountTooLow {
        /// The actual USD input amount
        usd_amount: f64,
    },

    /// Output amount is below minimum threshold ($0.10 USD)
    #[bad_request("bungee-output-amount-too-low", severity = "warn")]
    #[error("[bungee] bungee quote output amount too low: ${usd_amount:.2} (minimum $0.10)")]
    OutputAmountTooLow {
        /// The actual USD output amount
        usd_amount: f64,
    },
}

/// Input for getting a Bungee quote (Inbox)
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
    /// Receiver wallet address on destination chain
    pub receiver_address: Address,
    /// User wallet address on source chain (depositor)
    pub user_address: Address,
}

/// Output for getting a Bungee quote
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetQuoteOutput {
    /// Expected output amount (as reported by Bungee auto route)
    pub output_amount: U256,
    /// Inbox transaction target
    pub tx_to: Address,
    /// Inbox transaction value (wei)
    pub tx_value: U256,
    /// Inbox transaction calldata
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub tx_data: Vec<u8>,
    /// Optional approval spender
    pub approval_spender: Option<Address>,
    /// Optional approval amount
    pub approval_amount: Option<U256>,
    /// Optional provider quote id for follow-up (status/build)
    pub quote_id: Option<String>,
    /// Optional provider request hash for status lookup
    pub request_hash: Option<String>,
}

/// Controls which token list Bungee should return
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum TokenListKind {
    /// Return the trending subset of tokens (Bungee upstream default)
    #[serde(rename = "trending")]
    #[default]
    Trending,
    /// Return the full list of supported tokens
    #[serde(rename = "full")]
    Full,
}

impl TokenListKind {
    /// String representation expected by the Bungee API
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trending => "trending",
            Self::Full => "full",
        }
    }
}

/// Identifier used to poll the Bungee status endpoint
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusIdentifier {
    /// Auto-route request hash returned from submit/build
    RequestHash(String),
    /// Manual route source transaction hash
    TxHash(String),
    /// Alternate identifier, e.g. Permit2 submission id
    Id(String),
}

impl StatusIdentifier {
    /// Return the query parameter key expected by the public API
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self {
            Self::RequestHash(_) => "requestHash",
            Self::TxHash(_) => "txHash",
            Self::Id(_) => "id",
        }
    }

    /// Retrieve the underlying identifier value
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::RequestHash(value) | Self::TxHash(value) | Self::Id(value) => value,
        }
    }

    /// Convert into a [`GetStatusInput`]
    #[must_use]
    pub fn into_input(self) -> GetStatusInput {
        match self {
            Self::RequestHash(value) => GetStatusInput {
                request_hash: Some(value),
                ..GetStatusInput::default()
            },
            Self::TxHash(value) => GetStatusInput {
                tx_hash: Some(value),
                ..GetStatusInput::default()
            },
            Self::Id(value) => GetStatusInput {
                id: Some(value),
                ..GetStatusInput::default()
            },
        }
    }
}

/// Input parameters for fetching the token list
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetTokenListInput {
    /// Optional wallet address used to enrich balances
    pub user_address: Option<Address>,
    /// Optional chain id filter list
    pub chain_ids: Option<Vec<u128>>,
    /// Token list variant requested from Bungee
    #[serde(default)]
    pub list: TokenListKind,
}

/// Token metadata exposed through Guild
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenMetadata {
    /// Token contract address
    pub address: Address,
    /// Human readable token name
    pub name: String,
    /// Token ticker symbol
    pub symbol: String,
    /// Number of decimals used by the token
    pub decimals: u8,
    /// Optional token icon URL reported by Bungee
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_uri: Option<String>,
}

/// Output token list grouped by chain id
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetTokenListOutput {
    /// Mapping of chain id to the available tokens
    pub tokens: BTreeMap<u128, Vec<TokenMetadata>>,
}

/// Input payload for checking the status of a submitted bridge
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetStatusInput {
    /// Request hash returned by Bungee (auto routes / Permit2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_hash: Option<String>,
    /// Manual route source chain transaction hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    /// Alternate identifier accepted by the public API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl GetStatusInput {
    /// Create from a request hash
    #[must_use]
    pub fn from_request_hash(request_hash: impl Into<String>) -> Self {
        Self {
            request_hash: Some(request_hash.into()),
            ..Self::default()
        }
    }

    /// Create from a transaction hash
    #[must_use]
    pub fn from_tx_hash(tx_hash: impl Into<String>) -> Self {
        Self {
            tx_hash: Some(tx_hash.into()),
            ..Self::default()
        }
    }

    /// Create from an alternate identifier
    #[must_use]
    pub fn from_id(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            ..Self::default()
        }
    }

    /// Resolve the identifier following Bungee's priority rules
    pub fn identifier(&self) -> Result<StatusIdentifier, Error> {
        let pick = |value: &Option<String>, ctor: fn(String) -> StatusIdentifier| {
            value
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| ctor(s.to_owned()))
        };

        pick(&self.request_hash, StatusIdentifier::RequestHash)
            .or_else(|| pick(&self.tx_hash, StatusIdentifier::TxHash))
            .or_else(|| pick(&self.id, StatusIdentifier::Id))
            .ok_or(Error::MissingStatusIdentifier)
    }

    /// Build query pairs for the public API call
    pub fn to_query_pairs(&self) -> Result<Vec<(String, String)>, Error> {
        let identifier = self.identifier()?;
        Ok(vec![(
            identifier.key().to_string(),
            identifier.value().to_string(),
        )])
    }
}

/// Status history returned by the Guild API
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct GetStatusOutput {
    /// Bungee status entries ordered with the most recent first
    pub statuses: Vec<StatusEntry>,
}

impl GetStatusOutput {
    /// Return the most recent status entry
    #[must_use]
    pub fn latest(&self) -> Option<&StatusEntry> {
        self.statuses.first()
    }
}

/// Individual status entry in the history
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct StatusEntry {
    /// Numeric status code
    pub code: BungeeStatusCode,
    /// Optional status label provided by Bungee
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional destination transaction hash once broadcast
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_tx_hash: Option<String>,
}

/// Enumeration of Bungee status codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BungeeStatusCode {
    /// Request submitted; waiting for solver assignment
    Pending,
    /// Solver assigned and preparing execution
    Assigned,
    /// Solver completed source-chain extraction
    Extracted,
    /// Destination transaction broadcast and fulfilled
    Fulfilled,
    /// Settlement completed on both chains
    Settled,
    /// Request expired before completion
    Expired,
    /// Request cancelled (user/system)
    Cancelled,
    /// Request refunded to the origin
    Refunded,
    /// Unknown / forward compatible status code
    Unknown(u8),
}

impl BungeeStatusCode {
    /// Numeric representation used by Bungee
    #[must_use]
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Assigned => 1,
            Self::Extracted => 2,
            Self::Fulfilled => 3,
            Self::Settled => 4,
            Self::Expired => 5,
            Self::Cancelled => 6,
            Self::Refunded => 7,
            Self::Unknown(code) => code,
        }
    }

    /// Construct from the numeric representation
    #[must_use]
    pub fn from_u8(code: u8) -> Self {
        match code {
            0 => Self::Pending,
            1 => Self::Assigned,
            2 => Self::Extracted,
            3 => Self::Fulfilled,
            4 => Self::Settled,
            5 => Self::Expired,
            6 => Self::Cancelled,
            7 => Self::Refunded,
            other => Self::Unknown(other),
        }
    }

    /// Human-readable label for the status code
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Assigned => "ASSIGNED",
            Self::Extracted => "EXTRACTED",
            Self::Fulfilled => "FULFILLED",
            Self::Settled => "SETTLED",
            Self::Expired => "EXPIRED",
            Self::Cancelled => "CANCELLED",
            Self::Refunded => "REFUNDED",
            Self::Unknown(_) => "UNKNOWN",
        }
    }
}

impl serde::Serialize for BungeeStatusCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(self.as_u8())
    }
}

impl<'de> serde::Deserialize<'de> for BungeeStatusCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = u8::deserialize(deserializer)?;
        Ok(Self::from_u8(code))
    }
}

impl fmt::Display for BungeeStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(code) => write!(f, "UNKNOWN({code})"),
            other => f.write_str(other.as_str()),
        }
    }
}
