use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Request headers for `GET /api/v1/bungee/status`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Headers {}

/// Query parameters for `GET /api/v1/bungee/status`.
#[derive(Debug, Clone, Serialize)]
pub struct Query {
    /// `requestHash`
    #[serde(rename = "requestHash", skip_serializing_if = "Option::is_none")]
    pub request_hash: Option<String>,
    /// `txHash`
    #[serde(rename = "txHash", skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    /// `id`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Response body variants for `GET /api/v1/bungee/status`.
#[derive(Debug, Clone)]
pub enum ResponseEnum {
    /// Successful 200 response.
    Ok200(StatusResponse),
    /// Any non-200 response with raw body bytes.
    Unknown(u16, Bytes),
}

/// Response from the status endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusResponse {
    /// Success flag returned by the API.
    pub success: bool,
    /// Optional status message.
    pub message: Option<String>,
    /// Result array containing latest-to-oldest statuses.
    #[serde(default)]
    pub result: Vec<StatusEntry>,
}

/// Individual status entry returned by Bungee.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusEntry {
    /// Numeric status code.
    #[serde(rename = "bungeeStatusCode")]
    pub status_code: u8,
    /// Optional human-readable status.
    #[serde(rename = "bungeeStatus")]
    pub status: Option<String>,
    /// Optional destination data block.
    #[serde(rename = "destinationData")]
    pub destination: Option<DestinationData>,
}

/// Destination data container.
#[derive(Debug, Clone, Deserialize)]
pub struct DestinationData {
    /// Destination transaction hash, if broadcast.
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
}
