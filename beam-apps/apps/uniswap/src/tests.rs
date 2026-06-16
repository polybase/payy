use serde_json::json;

use crate::{
    ApprovalResponse, PlanContext, QuoteRequest, SwapArgs, SwapPlanInput, SwapResponse, SwapToken,
    UniswapTransaction, approval_spender, build_swap_plan, parse_quote, public_api_key,
    quote_payload, selector, swap_payload,
};

#[test]
fn parses_agent_safe_swap_args() {
    let args = SwapArgs::parse(&[
        "swap".to_string(),
        "USDC".to_string(),
        "ETH".to_string(),
        "10".to_string(),
        "--slippage-bps".to_string(),
        "25".to_string(),
        "--recipient".to_string(),
        "alice".to_string(),
        "--deadline-seconds".to_string(),
        "300".to_string(),
        "--unlimited-approval".to_string(),
    ])
    .expect("parse swap args");

    assert_eq!(args.sell_token, "USDC");
    assert_eq!(args.buy_token, "ETH");
    assert_eq!(args.slippage_bps, 25);
    assert_eq!(args.recipient.as_deref(), Some("alice"));
    assert_eq!(args.deadline_seconds, 300);
    assert!(args.unlimited_approval);
}

#[test]
fn parses_approval_spender_and_selector() {
    let data = concat!(
        "0x095ea7b3",
        "000000000000000000000000000000000022d473030f116ddee9f6b43ac78ba3",
        "000000000000000000000000000000000000000000000000000000000000000a",
    );

    assert_eq!(selector(data).as_deref(), Some("0x095ea7b3"));
    assert_eq!(
        approval_spender(data).as_deref(),
        Some("0x000000000022d473030f116ddee9f6b43ac78ba3")
    );
}

#[test]
fn quote_parser_rejects_wrong_chain_and_uniswapx() {
    let request = quote_request();
    let wrong_chain = json!({
        "amountOut": "100",
        "route": "classic",
        "tokenInChainId": 1,
    });
    parse_quote(wrong_chain, &request).expect_err("reject wrong chain");

    let uniswapx = json!({
        "amountOut": "100",
        "route": "UniswapX",
        "tokenInChainId": 8453,
        "tokenOutChainId": 8453,
    });
    parse_quote(uniswapx, &request).expect_err("reject UniswapX route");
}

#[test]
fn public_api_key_is_embed_ready() {
    let key = public_api_key();

    assert_eq!(key, key.trim());
    assert!(!key.contains('\n'));
    assert!(!key.contains('\r'));
}

#[test]
fn quote_payload_uses_current_uniswap_schema() {
    let payload = quote_payload(&quote_request());

    assert_eq!(
        payload["swapper"].as_str(),
        Some("0x3333333333333333333333333333333333333333")
    );
    assert_eq!(payload.get("walletAddress"), None);
    assert_eq!(payload["slippageTolerance"].as_f64(), Some(0.5));
    assert_eq!(payload["permitAmount"].as_str(), Some("EXACT"));
}

#[test]
fn quote_parser_accepts_nested_current_response() {
    let request = quote_request();
    let quote = parse_quote(
        json!({
            "requestId": "request-1",
            "routing": "CLASSIC",
            "quote": {
                "chainId": 8453,
                "input": {
                    "amount": "10000000",
                    "token": "0x1111111111111111111111111111111111111111",
                },
                "output": {
                    "amount": "100",
                    "minAmount": "99",
                    "token": "0x0000000000000000000000000000000000000000",
                },
                "quoteId": "quote-1",
                "routeString": "[V3] USDC -- 0.05% ETH",
                "swapper": "0x3333333333333333333333333333333333333333",
            }
        }),
        &request,
    )
    .expect("parse current quote response");

    assert_eq!(quote.amount_out, "100");
    assert_eq!(quote.minimum_amount_out.as_deref(), Some("99"));
    assert_eq!(quote.quote_id, "quote-1");
    assert_eq!(quote.route, "CLASSIC");
    assert_eq!(
        quote.quote["swapper"].as_str(),
        Some(request.wallet.as_str())
    );
}

#[test]
fn swap_payload_omits_legacy_wallet_address() {
    let quote = parse_quote(
        json!({
            "amountOut": "100",
            "quoteId": "quote-1",
            "route": "classic",
            "tokenInChainId": 8453,
            "tokenOutChainId": 8453,
            "tokenIn": "0x1111111111111111111111111111111111111111",
            "tokenOut": "0x0000000000000000000000000000000000000000",
        }),
        &quote_request(),
    )
    .expect("parse quote");

    let payload = swap_payload(&quote, "0x3333333333333333333333333333333333333333");

    assert_eq!(payload.get("walletAddress"), None);
    assert_eq!(payload["simulateTransaction"].as_bool(), Some(true));
    assert_eq!(payload["refreshGasPrice"].as_bool(), Some(true));
    assert_eq!(payload["quote"]["quoteId"].as_str(), Some("quote-1"));
}

#[test]
fn builds_approval_and_swap_action_plan() {
    let args = SwapArgs::parse(&[
        "swap".to_string(),
        "USDC".to_string(),
        "ETH".to_string(),
        "10".to_string(),
        "--min-receive".to_string(),
        "90".to_string(),
    ])
    .expect("parse args");
    let quote = parse_quote(
        json!({
            "amountOut": "100",
            "quoteId": "quote-1",
            "route": "classic",
            "tokenInChainId": 8453,
            "tokenOutChainId": 8453,
            "tokenIn": "0x1111111111111111111111111111111111111111",
            "tokenOut": "0x0000000000000000000000000000000000000000",
        }),
        &quote_request(),
    )
    .expect("parse quote");

    let plan = build_swap_plan(SwapPlanInput {
        allowance: Some("0".to_string()),
        amount_raw: "10000000".to_string(),
        args,
        buy: token("ETH", "0x0000000000000000000000000000000000000000", true),
        context: PlanContext {
            app_id: "uniswap".to_string(),
            app_version: "1.0.2".to_string(),
            chain: "base".to_string(),
            manifest_sha256: "sha256:manifest".to_string(),
            wallet: "0x3333333333333333333333333333333333333333".to_string(),
            wasm_sha256: "sha256:wasm".to_string(),
        },
        expires_at: 1_000,
        min_receive_raw: Some("90".to_string()),
        quote,
        sell: token("USDC", "0x1111111111111111111111111111111111111111", false),
        sell_balance: "10000000".to_string(),
        approval: Some(ApprovalResponse {
            transaction: Some(transaction("0x095ea7b3000000000000000000000000000000000022d473030f116ddee9f6b43ac78ba30000000000000000000000000000000000000000000000000000000000989680")),
        }),
        swap: SwapResponse {
            raw: json!({
                "gasFee": "123",
                "quote": {
                    "route": [
                        { "protocol": "V3", "tokenIn": "USDC", "tokenOut": "ETH" },
                    ],
                },
                "requestId": "swap-request-1",
                "transaction": { "to": "0x2222222222222222222222222222222222222222" },
            }),
            transaction: transaction("0x3593564c"),
        },
    })
    .expect("build action plan");

    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].kind, "erc20-approval");
    assert_eq!(plan.steps[1].kind, "transaction");
    assert_eq!(plan.steps[1].metadata.get("swap"), None);
    assert_eq!(plan.steps[1].metadata.get("quote"), None);
    assert_eq!(
        plan.steps[1].metadata["request_id"].as_str(),
        Some("swap-request-1")
    );
    assert_eq!(plan.steps[1].metadata["gas_fee"].as_str(), Some("123"));
    assert!(
        plan.bindings
            .iter()
            .any(|binding| binding.key == "quote_id")
    );
    assert!(
        plan.bindings
            .iter()
            .any(|binding| binding.key == "swap_calldata_hash")
    );
    assert!(plan.bindings.iter().any(|binding| binding.key == "router"));
}

fn quote_request() -> QuoteRequest {
    QuoteRequest {
        amount: "10000000".to_string(),
        chain_id: 8453,
        recipient: "0x3333333333333333333333333333333333333333".to_string(),
        slippage_bps: 50,
        token_in: "0x1111111111111111111111111111111111111111".to_string(),
        token_out: "0x0000000000000000000000000000000000000000".to_string(),
        wallet: "0x3333333333333333333333333333333333333333".to_string(),
    }
}

fn token(label: &str, address: &str, is_native: bool) -> SwapToken {
    SwapToken {
        address: address.to_string(),
        decimals: 18,
        is_native,
        label: label.to_string(),
    }
}

fn transaction(data: &str) -> UniswapTransaction {
    UniswapTransaction {
        data: data.to_string(),
        gas_limit: Some("100000".to_string()),
        gas_price: Some("1".to_string()),
        to: "0x2222222222222222222222222222222222222222".to_string(),
        value: "0".to_string(),
    }
}
