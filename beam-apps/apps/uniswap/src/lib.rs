mod api;
mod args;
mod error;
mod generated {
    include!(concat!(env!("OUT_DIR"), "/public_api_key.rs"));
}
mod host;
mod plan;

pub use api::{
    ApprovalResponse, QuoteRequest, QuoteResponse, SwapResponse, UniswapTransaction,
    approval_spender, check_approval_payload, find_transaction, parse_quote, quote_payload,
    selector, swap_payload,
};
pub use args::SwapArgs;
pub use error::{Error, Result};
pub use host::{ActionBinding, ActionPlan, ActionStep, GuestInvocation, PlanContext, SwapToken};
pub use plan::{SwapPlanInput, build_swap_plan};
use serde_json::{Value, json};

#[cfg(test)]
mod tests;

pub fn public_api_key() -> &'static str {
    generated::BEAM_UNISWAP_PUBLIC_API_KEY
}

#[unsafe(no_mangle)]
pub extern "C" fn beam_uniswap_public_api_key_ptr() -> *const u8 {
    public_api_key().as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn beam_uniswap_public_api_key_len() -> usize {
    public_api_key().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn beam_alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    core::mem::forget(buffer);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn beam_free(ptr: *mut u8, capacity: usize) {
    if ptr.is_null() || capacity == 0 {
        return;
    }
    unsafe {
        let _ = Vec::<u8>::from_raw_parts(ptr, 0, capacity);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn beam_app_main(input_ptr: *const u8, input_len: usize) -> u64 {
    let result = run_guest(input_ptr, input_len).unwrap_or_else(error_response);
    pack_response(result)
}

fn run_guest(input_ptr: *const u8, input_len: usize) -> Result<Value> {
    if input_ptr.is_null() {
        return Err(Error::InvalidHostResponse {
            reason: "guest invocation pointer is null".to_string(),
        });
    }
    let input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };
    let invocation =
        serde_json::from_slice::<GuestInvocation>(input).map_err(|err| Error::Serialization {
            reason: err.to_string(),
        })?;
    host::ensure_host_abi(&invocation)?;
    let command = invocation
        .args
        .first()
        .map(String::as_str)
        .ok_or_else(|| Error::UnsupportedCommand {
            command: "<missing>".to_string(),
        })?;
    match command {
        "swap" => {
            let plan = run_swap(invocation)?;
            Ok(json!({
                "kind": "action-plan",
                "plan": plan,
            }))
        }
        other => Err(Error::UnsupportedCommand {
            command: other.to_string(),
        }),
    }
}

fn run_swap(invocation: GuestInvocation) -> Result<ActionPlan> {
    let args = SwapArgs::parse(&invocation.args)?;
    let chain = invocation.metadata.chain.clone();
    let wallet = invocation.metadata.wallet.clone();
    let recipient = host::resolve_address(args.recipient.as_deref())?;
    let sell = host::token_metadata(&chain, &args.sell_token)?;
    let buy = host::token_metadata(&chain, &args.buy_token)?;
    let amount_raw = amount_to_raw(&args.amount, sell.decimals)?;
    let min_receive_raw = args
        .min_receive
        .as_deref()
        .map(|amount| amount_to_raw(amount, buy.decimals))
        .transpose()?;
    let quote_request = QuoteRequest {
        amount: amount_raw.clone(),
        chain_id: invocation.metadata.chain_id,
        recipient,
        slippage_bps: args.slippage_bps,
        token_in: sell.address.clone(),
        token_out: buy.address.clone(),
        wallet: wallet.clone(),
    };
    let quote = parse_quote(
        host::http_json(
            "POST",
            "https://trade-api.gateway.uniswap.org/v1/quote",
            &quote_payload(&quote_request),
        )?,
        &quote_request,
    )?;
    let approval = if sell.is_native {
        None
    } else {
        let value = host::http_json(
            "POST",
            "https://trade-api.gateway.uniswap.org/v1/check_approval",
            &check_approval_payload(&quote_request),
        )?;
        Some(ApprovalResponse {
            transaction: find_transaction(&value),
        })
    };
    let allowance = approval
        .as_ref()
        .and_then(|approval| approval.transaction.as_ref())
        .and_then(|transaction| approval_spender(&transaction.data))
        .map(|spender| host::allowance(&chain, &sell.address, &spender))
        .transpose()?;
    let swap_value = host::http_json(
        "POST",
        "https://trade-api.gateway.uniswap.org/v1/swap",
        &swap_payload(&quote, &wallet),
    )?;
    let mut swap = SwapResponse {
        transaction: find_transaction(&swap_value).ok_or_else(|| Error::InvalidUniswapResponse {
            reason: "swap response missing transaction".to_string(),
        })?,
        raw: swap_value,
    };
    if swap.transaction.gas_limit.is_none() || swap.transaction.gas_price.is_none() {
        let (gas_limit, gas_price) = host::gas(
            &chain,
            &swap.transaction.to,
            &swap.transaction.data,
            &swap.transaction.value,
        )?;
        swap.transaction.gas_limit = swap.transaction.gas_limit.or(gas_limit);
        swap.transaction.gas_price = swap.transaction.gas_price.or(Some(gas_price));
    }
    simulate_best_effort(&chain, approval.as_ref(), &swap);

    build_swap_plan(SwapPlanInput {
        allowance,
        amount_raw,
        args,
        buy,
        context: invocation.metadata.plan_context(),
        expires_at: invocation.metadata.now,
        min_receive_raw,
        quote,
        sell_balance: host::balance(&chain, &sell.address)?,
        sell,
        approval,
        swap,
    })
}

fn simulate_best_effort(chain: &str, approval: Option<&ApprovalResponse>, swap: &SwapResponse) {
    if let Some(transaction) = approval
        .and_then(|approval| approval.transaction.as_ref())
        .filter(|transaction| approval_spender(&transaction.data).is_some())
    {
        let spender = approval_spender(&transaction.data);
        if let Err(err) = host::simulate(
            chain,
            &transaction.to,
            &transaction.data,
            &transaction.value,
            spender.as_deref(),
        ) {
            let _ = host::diagnostic("warn", &format!("approval simulation skipped: {err}"));
        }
    }
    if let Err(err) = host::simulate(
        chain,
        &swap.transaction.to,
        &swap.transaction.data,
        &swap.transaction.value,
        None,
    ) {
        let _ = host::diagnostic("warn", &format!("swap simulation skipped: {err}"));
    }
}

fn amount_to_raw(amount: &str, decimals: u8) -> Result<String> {
    let amount = amount.trim();
    let (whole, fractional) = amount.split_once('.').unwrap_or((amount, ""));
    if whole.is_empty() || !whole.chars().all(|char| char.is_ascii_digit()) {
        return Err(Error::InvalidInteger {
            value: amount.to_string(),
        });
    }
    if fractional.len() > usize::from(decimals)
        || !fractional.chars().all(|char| char.is_ascii_digit())
    {
        return Err(Error::InvalidInteger {
            value: amount.to_string(),
        });
    }
    let mut digits = whole.to_string();
    digits.push_str(fractional);
    for _ in fractional.len()..usize::from(decimals) {
        digits.push('0');
    }
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        Ok("0".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn error_response(error: Error) -> Value {
    json!({
        "kind": "error",
        "message": error.to_string(),
    })
}

fn pack_response(value: Value) -> u64 {
    let bytes = serde_json::to_vec(&value).unwrap_or_else(|err| {
        format!(
            r#"{{"kind":"error","message":"[beam-app-uniswap] serialization failed: {}"}}"#,
            err
        )
        .into_bytes()
    });
    let ptr = beam_alloc(bytes.len());
    if ptr.is_null() {
        return 0;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    }
    ((ptr as u64) << 32) | bytes.len() as u64
}
