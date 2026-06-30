// lint-long-file-override allow-max-lines=300
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Request headers for `GET /v3/swap/quote`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Headers {}

/// Query parameters for `GET /v3/swap/quote`.
#[derive(Debug, Clone, Serialize)]
pub struct Query {
    /// `userOps`
    #[serde(rename = "userOps")]
    pub user_ops: String,
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
    /// `slippage`
    pub slippage: String,
}

/// Response body variants for `GET /v3/swap/quote`.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum ResponseEnum {
    /// Successful 200 response.
    Ok200(QuoteResponse),
    /// Any non-200 response with raw body bytes.
    Unknown(u16, Bytes),
}

/// Wrapped Socket Swap v3 quote response.
#[derive(Debug, Clone, Deserialize)]
pub struct QuoteResponse {
    /// Success flag returned by the API.
    pub success: bool,
    /// HTTP-style status code reported by Socket.
    #[serde(rename = "statusCode")]
    pub status_code: u64,
    /// Wrapped quote payload.
    pub result: QuoteResult,
    /// Optional upstream message.
    pub message: Option<String>,
}

/// Quote result payload.
#[derive(Debug, Clone, Deserialize)]
pub struct QuoteResult {
    /// Origin chain id.
    #[serde(rename = "originChainId")]
    pub origin_chain_id: u128,
    /// Destination chain id.
    #[serde(rename = "destinationChainId")]
    pub destination_chain_id: u128,
    /// User wallet address on the source chain.
    #[serde(rename = "userAddress")]
    pub user_address: String,
    /// Receiver wallet address on the destination chain.
    #[serde(rename = "receiverAddress")]
    pub receiver_address: String,
    /// Input amount and token details echoed by Socket.
    pub input: Input,
    /// Candidate routes.
    #[serde(default)]
    pub routes: Vec<Route>,
}

/// Input amount and token details echoed by Socket.
#[derive(Debug, Clone, Deserialize)]
pub struct Input {
    /// Input token details.
    pub token: Token,
    /// Input token amount.
    pub amount: String,
    /// USD value of the input amount.
    #[serde(rename = "valueInUsd")]
    pub value_in_usd: Option<f64>,
    /// Price per unit in USD.
    #[serde(rename = "priceInUsd")]
    pub price_in_usd: Option<f64>,
}

/// Token chain and address details.
#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    /// Token chain id.
    #[serde(rename = "chainId")]
    pub chain_id: u128,
    /// Token contract address.
    pub address: String,
}

/// Socket Swap v3 route candidate.
#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    /// Route user operation type.
    #[serde(rename = "userOp")]
    pub user_op: String,
    /// Socket quote id.
    #[serde(rename = "quoteId")]
    pub quote_id: String,
    /// Unix timestamp after which the route should not be submitted.
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<u64>,
    /// Output amount information.
    pub output: Output,
    /// Estimated time to complete, in seconds. Socket returns this as a JSON
    /// number that may be fractional (e.g. `9.5`), so it must be `f64`.
    #[serde(rename = "estimatedTime")]
    pub estimated_time: Option<f64>,
    /// Route tags used for tie-break ranking.
    #[serde(rename = "routeTags", default)]
    pub route_tags: Vec<String>,
    /// Optional approval details required before submitting txData.
    pub approval: Option<Approval>,
    /// Transaction data for the Socket route.
    #[serde(rename = "txData")]
    pub tx_data: TxData,
}

/// Output amount wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct Output {
    /// Output token details.
    pub token: Token,
    /// Expected output amount.
    pub amount: String,
    /// Guaranteed minimum output amount.
    #[serde(rename = "minAmountOut")]
    pub min_amount_out: String,
    /// Price per unit in USD.
    #[serde(rename = "priceInUsd")]
    pub price_in_usd: Option<f64>,
    /// USD value of the output amount.
    #[serde(rename = "valueInUsd")]
    pub value_in_usd: Option<f64>,
}

/// ERC-20 approval details.
#[derive(Debug, Clone, Deserialize)]
pub struct Approval {
    /// Spender address to approve.
    #[serde(rename = "spenderAddress")]
    pub spender_address: String,
    /// Allowance amount.
    pub amount: String,
    /// Approved token address.
    #[serde(rename = "tokenAddress")]
    pub token_address: String,
    /// User wallet address.
    #[serde(rename = "userAddress")]
    pub user_address: String,
}

/// Socket route transaction data container.
#[derive(Debug, Clone, Deserialize)]
pub struct TxData {
    /// Transaction data kind.
    pub kind: String,
    /// EVM transaction object.
    pub object: TxObject,
}

/// EVM transaction object returned under `txData.object`.
#[derive(Debug, Clone, Deserialize)]
pub struct TxObject {
    /// Source chain id.
    #[serde(rename = "chainId")]
    pub chain_id: u128,
    /// Transaction target.
    pub to: String,
    /// Calldata.
    pub data: String,
    /// Native value.
    pub value: String,
}
