use serde_json::{Number, Value, json};

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
    pub gas_price_hint: Option<String>,
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
        "permitAmount": "EXACT",
        "protocols": ["V2", "V3", "V4"],
        "recipient": request.recipient,
        "routingPreference": "BEST_PRICE",
        "slippageTolerance": slippage_tolerance(request.slippage_bps),
        "swapper": request.wallet,
        "tokenIn": request.token_in,
        "tokenInChainId": request.chain_id,
        "tokenOut": request.token_out,
        "tokenOutChainId": request.chain_id,
        "type": "EXACT_INPUT",
        "urgency": "normal",
    })
}

pub fn swap_payload(quote: &QuoteResponse, _wallet: &str) -> Value {
    json!({
        "quote": quote.quote,
        "refreshGasPrice": true,
        "simulateTransaction": true,
        "urgency": "normal",
    })
}

pub fn parse_quote(value: Value, request: &QuoteRequest) -> Result<QuoteResponse> {
    let quote = value.get("quote").cloned().unwrap_or_else(|| value.clone());
    validate_optional_field(
        &quote,
        &["tokenInChainId", "chainId"],
        &[],
        &request.chain_id.to_string(),
    )?;
    validate_optional_field(
        &quote,
        &["tokenOutChainId"],
        &[],
        &request.chain_id.to_string(),
    )?;
    validate_optional_field(
        &quote,
        &["tokenIn", "inputToken"],
        &[&["input", "token"]],
        &request.token_in,
    )?;
    validate_optional_field(
        &quote,
        &["tokenOut", "outputToken"],
        &[&["output", "token"]],
        &request.token_out,
    )?;
    let amount_out = first_string_or_path(
        &quote,
        &["amountOut", "quoteAmount"],
        &[&["output", "amount"]],
    )
    .ok_or_else(|| Error::InvalidUniswapResponse {
        reason: "quote missing output amount".to_string(),
    })?;
    let quote_id = first_string(&quote, &["quoteId", "requestId", "routingId"])
        .or_else(|| first_string(&value, &["quoteId", "requestId", "routingId"]))
        .unwrap_or_else(|| "uniswap-quote".to_string());
    let route = first_string(&value, &["routing"])
        .or_else(|| first_string(&quote, &["routing", "routeString", "route"]))
        .unwrap_or_else(|| "classic".to_string());
    if is_order_route(&route) {
        return Err(Error::UnsupportedUniswapRoute { route });
    }

    Ok(QuoteResponse {
        amount_out,
        minimum_amount_out: first_string_or_path(
            &quote,
            &["amountOutMinimum", "minimumAmountOut", "minAmountOut"],
            &[&["output", "minAmount"]],
        ),
        quote,
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

fn validate_optional_field(
    value: &Value,
    keys: &[&str],
    paths: &[&[&str]],
    expected: &str,
) -> Result<()> {
    let Some(actual) = first_string_or_path(value, keys, paths) else {
        return Ok(());
    };
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(Error::InvalidUniswapResponse {
            reason: format!("quote field mismatch: expected {expected}, got {actual}"),
        });
    }

    Ok(())
}

fn slippage_tolerance(slippage_bps: u32) -> Value {
    let value = f64::from(slippage_bps) / 100.0;
    Value::Number(Number::from_f64(value).unwrap_or_else(|| Number::from(0)))
}

fn is_order_route(route: &str) -> bool {
    let route = route.to_ascii_lowercase();
    route.contains("dutch") || route.contains("uniswapx") || route == "priority"
}

fn parse_transaction(value: &Value) -> Option<UniswapTransaction> {
    Some(UniswapTransaction {
        data: first_string(value, &["data", "calldata", "input"])?,
        gas_limit: first_string(value, &["gasLimit", "gas"]),
        gas_price_hint: first_string(value, &["gasPrice", "maxFeePerGas"]),
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

fn first_string_or_path(value: &Value, keys: &[&str], paths: &[&[&str]]) -> Option<String> {
    first_string(value, keys).or_else(|| paths.iter().find_map(|path| path_string(value, path)))
}

fn path_string(value: &Value, path: &[&str]) -> Option<String> {
    let value = path.iter().try_fold(value, |value, key| value.get(key))?;
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}
