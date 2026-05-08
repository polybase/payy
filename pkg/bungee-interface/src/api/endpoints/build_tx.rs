use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::api::endpoints::quote::{ApprovalData, TxData};

/// Request headers for `GET /api/v1/bungee/build-tx`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Headers {}

/// Query parameters for `GET /api/v1/bungee/build-tx`.
#[derive(Debug, Clone, Serialize)]
pub struct Query {
    /// `quoteId`
    #[serde(rename = "quoteId")]
    pub quote_id: String,
}

/// Response body variants for `GET /api/v1/bungee/build-tx`.
#[derive(Debug, Clone)]
pub enum ResponseEnum {
    /// Successful 200 response.
    Ok200(BuildTxResponse),
    /// Any non-200 response with raw body bytes.
    Unknown(u16, Bytes),
}

/// Build-tx response returned when constructing a transaction.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildTxResponse {
    /// The wrapped result object.
    pub result: BuildTxResult,
}

/// Subset of fields consumed from the build-tx result.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildTxResult {
    /// Transaction data for the Inbox call.
    #[serde(rename = "txData")]
    pub txn: TxData,
    /// Optional approval details required prior to the Inbox call.
    #[serde(rename = "approvalData")]
    pub approval: Option<ApprovalData>,
    /// Optional provider quote id.
    #[serde(rename = "quoteId")]
    pub quote_id: Option<String>,
    /// Optional provider request hash.
    #[serde(rename = "requestHash")]
    pub request_hash: Option<String>,
}
