use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Request headers for `GET /api/v1/tokens/list`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Headers {}

/// Query parameters for `GET /api/v1/tokens/list`.
#[derive(Debug, Clone, Serialize)]
pub struct Query {
    /// `list`
    #[serde(rename = "list")]
    pub list: String,
    /// `userAddress`
    #[serde(rename = "userAddress", skip_serializing_if = "Option::is_none")]
    pub user_address: Option<String>,
    /// `chainIds`
    #[serde(rename = "chainIds", skip_serializing_if = "Option::is_none")]
    pub chain_ids: Option<String>,
}

/// Response body variants for `GET /api/v1/tokens/list`.
#[derive(Debug, Clone)]
pub enum ResponseEnum {
    /// Successful 200 response.
    Ok200(TokenListResponse),
    /// Any non-200 response with raw body bytes.
    Unknown(u16, Bytes),
}

/// Token list response envelope returned by Bungee.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenListResponse {
    /// Indicates whether the upstream request succeeded.
    pub success: bool,
    /// HTTP-style status code reported by Bungee.
    #[serde(rename = "statusCode")]
    pub status_code: u64,
    /// Wrapped token list payload.
    pub result: TokenListResult,
}

/// Result wrapper containing tokens grouped by chain id.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenListResult {
    /// Map of chain id string to token entries.
    #[serde(flatten)]
    pub tokens: BTreeMap<String, Vec<TokenListToken>>,
}

/// Token metadata entry as returned by the Bungee API.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenListToken {
    /// Chain id reported in the entry.
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    /// Token contract address.
    pub address: String,
    /// Human-readable token name.
    pub name: String,
    /// Token symbol ticker.
    pub symbol: String,
    /// Token decimals.
    pub decimals: u8,
    /// Optional logo URI for the token.
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    /// Whether the token is shortlisted.
    #[serde(rename = "isShortListed")]
    pub is_short_listed: bool,
    /// Optional trending rank for the token.
    #[serde(rename = "trendingRank")]
    pub trending_rank: Option<u64>,
    /// Optional token market cap in USD.
    #[serde(rename = "marketCap")]
    pub market_cap: Option<f64>,
    /// Optional total volume in USD.
    #[serde(rename = "totalVolume")]
    pub total_volume: Option<f64>,
    /// Token balance string for the provided user address.
    pub balance: String,
    /// USD balance for the provided user address.
    #[serde(rename = "balanceInUsd")]
    pub balance_in_usd: f64,
    /// Metadata tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether the token is verified.
    #[serde(rename = "isVerified")]
    pub is_verified: bool,
}
