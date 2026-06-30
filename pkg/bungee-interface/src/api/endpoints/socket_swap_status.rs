use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Request headers for `GET /v3/swap/status`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Headers {}

/// Query parameters for `GET /v3/swap/status`.
#[derive(Debug, Clone, Serialize)]
pub struct Query {
    /// `quoteId`
    #[serde(rename = "quoteId", skip_serializing_if = "Option::is_none")]
    pub quote_id: Option<String>,
    /// `srcTxHash`
    #[serde(rename = "srcTxHash", skip_serializing_if = "Option::is_none")]
    pub src_tx_hash: Option<String>,
}

/// Response body variants for `GET /v3/swap/status`.
#[derive(Debug, Clone)]
pub enum ResponseEnum {
    /// Successful 200 response.
    Ok200(StatusResponse),
    /// Any non-200 response with raw body bytes.
    Unknown(u16, Bytes),
}

/// Wrapped Socket Swap v3 status response.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusResponse {
    /// Success flag returned by the API.
    pub success: bool,
    /// HTTP-style status code reported by Socket.
    #[serde(rename = "statusCode")]
    pub status_code: u64,
    /// Wrapped status payload.
    pub result: StatusResult,
    /// Optional upstream message.
    pub message: Option<String>,
}

/// Socket status result payload.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusResult {
    /// Socket quote id.
    #[serde(rename = "quoteId")]
    pub quote_id: String,
    /// Route user operation type.
    #[serde(rename = "userOp")]
    pub user_op: String,
    /// Coarse status.
    pub status: String,
    /// Granular status code.
    #[serde(rename = "statusCode")]
    pub status_code: String,
    /// Destination leg data.
    pub destination: Option<StatusDestination>,
}

/// Destination leg status data.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusDestination {
    /// Destination transaction hash, if broadcast.
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
}
