// lint-long-file-override allow-max-lines=260
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Request headers for `GET /api/v1/bungee/quote`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Headers {}

/// Query parameters for `GET /api/v1/bungee/quote`.
#[derive(Debug, Clone, Serialize)]
pub struct Query {
    /// `originChainId`
    #[serde(rename = "originChainId")]
    pub origin_chain_id: String,
    /// `destinationChainId`
    #[serde(rename = "destinationChainId")]
    pub destination_chain_id: String,
    /// `inputToken`
    #[serde(rename = "inputToken")]
    pub input_token: String,
    /// `outputToken`
    #[serde(rename = "outputToken")]
    pub output_token: String,
    /// `inputAmount`
    #[serde(rename = "inputAmount")]
    pub input_amount: String,
    /// `receiverAddress`
    #[serde(rename = "receiverAddress")]
    pub receiver_address: String,
    /// `userAddress`
    #[serde(rename = "userAddress")]
    pub user_address: String,
    /// `useInbox`
    #[serde(rename = "useInbox", skip_serializing_if = "Option::is_none")]
    pub use_inbox: Option<String>,
    /// `refuel`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refuel: Option<String>,
    /// `excludeBridges`
    #[serde(rename = "excludeBridges", skip_serializing_if = "Option::is_none")]
    pub exclude_bridges: Option<String>,
    /// `excludeDexes`
    #[serde(rename = "excludeDexes", skip_serializing_if = "Option::is_none")]
    pub exclude_dexes: Option<String>,
    /// `enableManual`
    #[serde(rename = "enableManual", skip_serializing_if = "Option::is_none")]
    pub enable_manual: Option<String>,
}

/// Response body variants for `GET /api/v1/bungee/quote`.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum ResponseEnum {
    /// Successful 200 response.
    Ok200(QuoteResponse),
    /// Any non-200 response with raw body bytes.
    Unknown(u16, Bytes),
}

/// Top-level Bungee quote response returned by the public API.
#[derive(Debug, Clone, Deserialize)]
pub struct QuoteResponse {
    /// The wrapped result object.
    pub result: QuoteResult,
}

/// Quote result wrapper containing the auto and manual routes.
#[derive(Debug, Clone, Deserialize)]
pub struct QuoteResult {
    /// Input amount information.
    pub input: Option<Input>,
    /// Auto route details.
    #[serde(rename = "autoRoute")]
    pub auto_route: Option<AutoRoute>,
    /// Optional list of manual routes.
    #[serde(rename = "manualRoutes")]
    pub manual_routes: Option<Vec<ManualRoute>>,
}

/// Input amount information from the Bungee quote response.
#[derive(Debug, Clone, Deserialize)]
pub struct Input {
    /// Input token amount.
    pub amount: String,
    /// USD value of the input amount.
    #[serde(rename = "valueInUsd")]
    pub value_in_usd: Option<f64>,
    /// Price per unit in USD.
    #[serde(rename = "priceInUsd")]
    pub price_in_usd: Option<f64>,
}

/// Auto route contents used to build the Inbox transaction.
#[derive(Debug, Clone, Deserialize)]
pub struct AutoRoute {
    /// Transaction data for the Inbox call.
    #[serde(rename = "txData")]
    pub txn: TxData,
    /// Optional approval details required prior to the Inbox call.
    #[serde(rename = "approvalData")]
    pub approval: Option<ApprovalData>,
    /// Output amount information.
    pub output: Output,
    /// Estimated time to complete.
    #[serde(rename = "estimatedTime")]
    pub estimated_time: Option<u64>,
    /// Optional provider quote id.
    #[serde(rename = "quoteId")]
    pub quote_id: Option<String>,
    /// Optional provider request hash.
    #[serde(rename = "requestHash")]
    pub request_hash: Option<String>,
}

/// Manual route contents used to build the Inbox transaction.
#[derive(Debug, Clone, Deserialize)]
pub struct ManualRoute {
    /// Transaction data for the Inbox call.
    #[serde(rename = "txData")]
    pub txn: Option<TxData>,
    /// Optional approval details required prior to the Inbox call.
    #[serde(rename = "approvalData")]
    pub approval: Option<ApprovalData>,
    /// Output amount information.
    pub output: Output,
    /// Estimated time to complete.
    #[serde(rename = "estimatedTime")]
    pub estimated_time: Option<u64>,
    /// Optional provider quote id.
    #[serde(rename = "quoteId")]
    pub quote_id: Option<String>,
    /// Optional provider request hash.
    #[serde(rename = "requestHash")]
    pub request_hash: Option<String>,
}

/// Transaction data returned by Bungee for Inbox execution.
#[derive(Debug, Clone, Deserialize)]
pub struct TxData {
    /// Inbox contract address.
    pub to: String,
    /// Calldata for the Inbox call.
    pub data: String,
    /// Ether value for the Inbox call.
    pub value: String,
}

/// ERC-20 approval details.
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalData {
    /// Spender address to approve.
    #[serde(rename = "spenderAddress")]
    pub spender: String,
    /// Allowance amount.
    pub amount: String,
}

/// Output amount wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct Output {
    /// Expected output amount.
    pub amount: String,
    /// USD value of the output amount.
    #[serde(rename = "valueInUsd")]
    pub value_in_usd: Option<f64>,
    /// Effective output USD amount.
    #[serde(rename = "effectiveValueInUsd")]
    pub effective_value_in_usd: Option<f64>,
}
