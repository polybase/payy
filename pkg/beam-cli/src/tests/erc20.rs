use serde_json::json;

use crate::commands::erc20::render_balance_output;

#[test]
fn erc20_balance_output_includes_token_address_and_compact_omits_the_token_label() {
    let output = render_balance_output(
        "base",
        "USDC",
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        "0x740747e7e3a1e112",
        "12.5",
        6,
        "12500000",
    );

    assert_eq!(output.compact.as_deref(), Some("12.5"));
    assert_eq!(
        output.default,
        "12.5 USDC\nAddress: 0x740747e7e3a1e112\nToken: 0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
    );
    assert_eq!(
        &output.value,
        &json!({
            "address": "0x740747e7e3a1e112",
            "balance": "12.5",
            "chain": "base",
            "decimals": 6,
            "token": "USDC",
            "token_address": "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
            "value": "12500000",
        })
    );
}
