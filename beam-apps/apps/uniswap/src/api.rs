use serde_json::{Value, json};

use crate::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalResponse {
    pub transaction: Option<UniswapTransaction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuoteResponse {
    pub amount_out: String,
    pub minimum_amount_out: Option<String>,
    pub quote: Value,
    pub quote_id: String,
    pub route: String,
    pub valid_for_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapResponse {
    pub raw: Value,
    pub transaction: UniswapTransaction,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct UniswapTransaction {
    pub data: String,
    pub gas_limit: Option<String>,
    pub gas_price: Option<String>,
    pub to: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuoteRequest {
    pub amount: String,
    pub chain_id: u64,
    pub recipient: String,
    pub slippage_bps: u32,
    pub token_in: String,
    pub token_out: String,
    pub wallet: String,
}

pub fn check_approval_payload(request: &QuoteRequest) -> Value {
    json!({
        "amount": request.amount,
        "chainId": request.chain_id,
        "token": request.token_in,
        "walletAddress": request.wallet,
    })
}

pub fn quote_payload(request: &QuoteRequest) -> Value {
    json!({
        "amount": request.amount,
        "protocols": ["V2", "V3", "V4"],
        "recipient": request.recipient,
        "slippageTolerance": request.slippage_bps,
        "tokenIn": request.token_in,
        "tokenInChainId": request.chain_id,
        "tokenOut": request.token_out,
        "tokenOutChainId": request.chain_id,
        "type": "EXACT_INPUT",
        "walletAddress": request.wallet,
    })
}

pub fn swap_payload(quote: &QuoteResponse, wallet: &str) -> Value {
    json!({
        "quote": quote.quote,
        "simulateTransaction": true,
        "walletAddress": wallet,
    })
}

pub fn parse_quote(value: Value, request: &QuoteRequest) -> Result<QuoteResponse> {
    validate_optional_field(
        &value,
        &["tokenInChainId", "chainId"],
        &request.chain_id.to_string(),
    )?;
    validate_optional_field(&value, &["tokenOutChainId"], &request.chain_id.to_string())?;
    validate_optional_field(&value, &["tokenIn", "inputToken"], &request.token_in)?;
    validate_optional_field(&value, &["tokenOut", "outputToken"], &request.token_out)?;
    let amount_out =
        first_string(&value, &["amountOut", "output", "quoteAmount"]).ok_or_else(|| {
            Error::InvalidUniswapResponse {
                reason: "quote missing output amount".to_string(),
            }
        })?;
    let quote_id = first_string(&value, &["quoteId", "requestId", "routingId"])
        .unwrap_or_else(|| "uniswap-quote".to_string());
    let route = first_string(&value, &["routing", "routeString", "route"])
        .unwrap_or_else(|| "classic".to_string());
    if route.to_ascii_lowercase().contains("dutch")
        || route.to_ascii_lowercase().contains("uniswapx")
    {
        return Err(Error::UnsupportedUniswapRoute { route });
    }

    Ok(QuoteResponse {
        amount_out,
        minimum_amount_out: first_string(
            &value,
            &["amountOutMinimum", "minimumAmountOut", "minAmountOut"],
        ),
        quote: value,
        quote_id,
        route,
        valid_for_seconds: 180,
    })
}

pub fn find_transaction(value: &Value) -> Option<UniswapTransaction> {
    [
        "approval",
        "approvalTransaction",
        "swap",
        "transaction",
        "tx",
    ]
    .iter()
    .find_map(|key| value.get(key))
    .or(Some(value))
    .and_then(parse_transaction)
}

pub fn selector(data: &str) -> Option<String> {
    let data = data.strip_prefix("0x").unwrap_or(data);
    (data.len() >= 8).then(|| format!("0x{}", &data[..8]))
}

pub fn approval_spender(data: &str) -> Option<String> {
    let data = data.strip_prefix("0x").unwrap_or(data);
    if data.len() < 8 + 64 || &data[..8].to_ascii_lowercase() != "095ea7b3" {
        return None;
    }
    Some(format!("0x{}", &data[8 + 24..8 + 64]))
}

fn validate_optional_field(value: &Value, keys: &[&str], expected: &str) -> Result<()> {
    let Some(actual) = first_string(value, keys) else {
        return Ok(());
    };
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(Error::InvalidUniswapResponse {
            reason: format!("quote field mismatch: expected {expected}, got {actual}"),
        });
    }

    Ok(())
}

fn parse_transaction(value: &Value) -> Option<UniswapTransaction> {
    Some(UniswapTransaction {
        data: first_string(value, &["data", "calldata", "input"])?,
        gas_limit: first_string(value, &["gasLimit", "gas"]),
        gas_price: first_string(value, &["gasPrice", "maxFeePerGas"]),
        to: first_string(value, &["to", "target"])?,
        value: first_string(value, &["value"]).unwrap_or_else(|| "0".to_string()),
    })
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(key).and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
    })
}
