use contracts::Address;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Controls which token list Bungee should return.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum TokenListKind {
    /// Return the trending subset of tokens.
    #[serde(rename = "trending")]
    #[default]
    Trending,
    /// Return the full list of supported tokens.
    #[serde(rename = "full")]
    Full,
}

impl TokenListKind {
    /// String representation expected by the Bungee API.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trending => "trending",
            Self::Full => "full",
        }
    }
}

/// Input parameters for fetching the token list.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetTokenListInput {
    /// Optional wallet address used to enrich balances.
    pub user_address: Option<Address>,
    /// Optional chain id filter list.
    pub chain_ids: Option<Vec<u128>>,
    /// Token list variant requested from Bungee.
    #[serde(default)]
    pub list: TokenListKind,
}

/// Token metadata exposed through Guild.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenMetadata {
    /// Token contract address.
    pub address: Address,
    /// Human readable token name.
    pub name: String,
    /// Token ticker symbol.
    pub symbol: String,
    /// Number of decimals used by the token.
    pub decimals: u8,
    /// Optional token icon URL reported by Bungee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_uri: Option<String>,
}

/// Output token list grouped by chain id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetTokenListOutput {
    /// Mapping of chain id to the available tokens.
    pub tokens: BTreeMap<u128, Vec<TokenMetadata>>,
}
