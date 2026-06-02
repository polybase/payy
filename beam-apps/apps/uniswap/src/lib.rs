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
pub use host::{ActionBinding, ActionPlan, ActionStep, PlanContext, SwapToken};
pub use plan::{SwapPlanInput, build_swap_plan};

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
pub extern "C" fn beam_app_main() {
    let _ = core::hint::black_box(public_api_key());
}
