use num_bigint::BigUint;
use num_traits::Num;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{ActionBinding, ActionPlan, ActionStep, PlanContext, SwapToken};
use crate::{
    ApprovalResponse, Error, QuoteResponse, Result, SwapArgs, SwapResponse, UniswapTransaction,
    approval_spender, selector,
};

const APPROVAL_TTL_SECONDS: u64 = 15 * 60;
const MAX_U256_HEX: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

#[derive(Clone, Debug)]
pub struct SwapPlanInput {
    pub allowance: Option<String>,
    pub amount_raw: String,
    pub args: SwapArgs,
    pub buy: SwapToken,
    pub context: PlanContext,
    pub expires_at: u64,
    pub min_receive_raw: Option<String>,
    pub quote: QuoteResponse,
    pub sell: SwapToken,
    pub sell_balance: String,
    pub approval: Option<ApprovalResponse>,
    pub swap: SwapResponse,
}

pub fn build_swap_plan(input: SwapPlanInput) -> Result<ActionPlan> {
    ensure_balance(&input)?;
    ensure_min_receive(&input.quote.amount_out, input.min_receive_raw.as_deref())?;
    ensure_max_gas(&input.swap.transaction, input.args.max_gas.as_deref())?;
    if input.quote.valid_for_seconds == 0 {
        return Err(Error::QuoteExpired);
    }

    let mut steps = Vec::new();
    let approval_spender = input
        .approval
        .as_ref()
        .and_then(|response| response.transaction.as_ref())
        .and_then(|transaction| approval_spender(&transaction.data));
    if let Some(step) = approval_step(&input)? {
        steps.push(step);
    }
    steps.push(swap_step(&input));

    let expires_at = input.expires_at + input.quote.valid_for_seconds.min(APPROVAL_TTL_SECONDS);
    let command = format!(
        "swap {} {} {}",
        input.args.sell_token, input.args.buy_token, input.args.amount
    );
    let bindings = uniswap_bindings(&input, approval_spender.as_deref(), expires_at);
    let constraints = plan_constraints(&input);
    Ok(ActionPlan {
        app_id: input.context.app_id,
        app_version: input.context.app_version,
        wasm_sha256: input.context.wasm_sha256,
        manifest_sha256: input.context.manifest_sha256,
        command,
        wallet: Some(input.context.wallet),
        chain: input.context.chain,
        steps,
        bindings,
        constraints,
        expires_at,
    })
}

fn approval_step(input: &SwapPlanInput) -> Result<Option<ActionStep>> {
    if input.sell.is_native {
        return Ok(None);
    }
    let allowance = input.allowance.as_deref().unwrap_or("0");
    if parse_uint(allowance)? >= parse_uint(&input.amount_raw)? {
        return Ok(None);
    }
    let Some(mut transaction) = input
        .approval
        .as_ref()
        .and_then(|response| response.transaction.clone())
    else {
        return Err(Error::InvalidUniswapResponse {
            reason: "approval response missing transaction".to_string(),
        });
    };
    let spender =
        approval_spender(&transaction.data).ok_or_else(|| Error::InvalidUniswapResponse {
            reason: "approval transaction missing spender".to_string(),
        })?;
    if input.args.unlimited_approval {
        transaction.data = unlimited_approval_data(&spender)?;
    }
    let value = if input.args.unlimited_approval {
        parse_uint(&format!("0x{MAX_U256_HEX}"))?.to_string()
    } else {
        input.amount_raw.clone()
    };

    Ok(Some(ActionStep {
        kind: "erc20-approval".to_string(),
        metadata: json!({
            "approval_mode": if input.args.unlimited_approval { "unlimited" } else { "exact" },
            "sell_token": input.sell.label,
            "transaction": transaction_json(&transaction),
        }),
        selector: selector(&transaction.data),
        spender: Some(spender),
        summary: format!(
            "Approve {} {} for Uniswap{}",
            if input.args.unlimited_approval {
                "unlimited".to_string()
            } else {
                input.args.amount.clone()
            },
            input.sell.label,
            if input.args.unlimited_approval {
                " (higher risk)"
            } else {
                ""
            },
        ),
        target: Some(transaction.to),
        value: Some(value),
    }))
}

fn swap_step(input: &SwapPlanInput) -> ActionStep {
    let transaction = &input.swap.transaction;
    ActionStep {
        kind: "transaction".to_string(),
        metadata: swap_metadata(input, transaction),
        selector: selector(&transaction.data),
        spender: None,
        summary: format!(
            "Swap {} {} for {}",
            input.args.amount, input.sell.label, input.buy.label
        ),
        target: Some(transaction.to.clone()),
        value: Some(transaction.value.clone()),
    }
}

fn swap_metadata(input: &SwapPlanInput, transaction: &UniswapTransaction) -> Value {
    let mut metadata = json!({
        "buy": input.buy.label,
        "quote_id": input.quote.quote_id,
        "route": input.quote.route,
        "sell": input.sell.label,
        "slippage_bps": input.args.slippage_bps,
        "transaction": transaction_json(transaction),
    });
    if let Some(request_id) = raw_string(&input.swap.raw, "requestId") {
        metadata["request_id"] = json!(request_id);
    }
    if let Some(gas_fee) = raw_string(&input.swap.raw, "gasFee") {
        metadata["gas_fee"] = json!(gas_fee);
    }

    metadata
}

fn raw_string(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn ensure_balance(input: &SwapPlanInput) -> Result<()> {
    if parse_uint(&input.sell_balance)? < parse_uint(&input.amount_raw)? {
        return Err(Error::InsufficientBalance {
            token: input.sell.label.clone(),
        });
    }
    Ok(())
}

fn ensure_min_receive(amount_out: &str, min_receive: Option<&str>) -> Result<()> {
    let Some(min_receive) = min_receive else {
        return Ok(());
    };
    if parse_uint(amount_out)? < parse_uint(min_receive)? {
        return Err(Error::InvalidArgument {
            reason: "quote below minimum receive".to_string(),
        });
    }
    Ok(())
}

fn ensure_max_gas(transaction: &UniswapTransaction, max_gas: Option<&str>) -> Result<()> {
    let (Some(gas_limit), Some(max_gas)) = (transaction.gas_limit.as_deref(), max_gas) else {
        return Ok(());
    };
    if parse_uint(gas_limit)? > parse_uint(max_gas)? {
        return Err(Error::InvalidArgument {
            reason: "swap gas estimate exceeds max gas".to_string(),
        });
    }
    Ok(())
}

fn transaction_json(transaction: &UniswapTransaction) -> Value {
    json!({
        "data": transaction.data,
        "gas_limit": transaction.gas_limit,
        "gas_price": transaction.gas_price,
        "to": transaction.to,
        "value": transaction.value,
    })
}

fn plan_constraints(input: &SwapPlanInput) -> Vec<String> {
    let mut constraints = vec![
        format!("slippage_bps={}", input.args.slippage_bps),
        format!("deadline_seconds={}", input.args.deadline_seconds),
        format!("quoted_amount_out={}", input.quote.amount_out),
    ];
    if let Some(minimum_amount_out) = &input.quote.minimum_amount_out {
        constraints.push(format!("quote_minimum_amount_out={minimum_amount_out}"));
    }
    if let Some(min_receive) = &input.args.min_receive {
        constraints.push(format!("min_receive={min_receive}"));
    }
    if let Some(max_gas) = &input.args.max_gas {
        constraints.push(format!("max_gas={max_gas}"));
    }
    constraints
}

fn uniswap_bindings(
    input: &SwapPlanInput,
    spender: Option<&str>,
    expires_at: u64,
) -> Vec<ActionBinding> {
    let mut bindings = vec![
        binding("quote_id", &input.quote.quote_id),
        binding("quote_expires_at", &expires_at.to_string()),
        binding("route_hash", &sha256_hex(&input.quote.route)),
        binding(
            "swap_calldata_hash",
            &sha256_hex(&input.swap.transaction.data),
        ),
        binding("router", &input.swap.transaction.to),
        binding("sell_token", &input.sell.address),
        binding("buy_token", &input.buy.address),
        binding("amount_in", &input.amount_raw),
        binding("amount_out", &input.quote.amount_out),
    ];
    if let Some(spender) = spender {
        bindings.push(binding("spender", spender));
    }

    bindings
}

fn binding(key: &str, value: &str) -> ActionBinding {
    ActionBinding {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn sha256_hex(value: &str) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(value.as_bytes())))
}

fn unlimited_approval_data(spender: &str) -> Result<String> {
    let spender = address_word(spender)?;
    Ok(format!("0x095ea7b3{spender}{MAX_U256_HEX}"))
}

fn address_word(address: &str) -> Result<String> {
    let address = address.strip_prefix("0x").unwrap_or(address);
    if address.len() != 40 || !address.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(Error::InvalidAddress {
            value: address.to_string(),
        });
    }
    Ok(format!("{address:0>64}").to_ascii_lowercase())
}

fn parse_uint(value: &str) -> Result<BigUint> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix("0x") {
        return BigUint::from_str_radix(hex, 16).map_err(|_| Error::InvalidInteger {
            value: value.to_string(),
        });
    }
    BigUint::from_str_radix(value, 10).map_err(|_| Error::InvalidInteger {
        value: value.to_string(),
    })
}
