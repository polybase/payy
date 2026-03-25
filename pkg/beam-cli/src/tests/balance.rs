use serde_json::json;

use crate::commands::balance::render_balance_output;

#[test]
fn compact_balance_output_omits_the_native_symbol() {
    let output = render_balance_output(
        "payy-dev",
        "PUSD",
        "http://127.0.0.1:8546",
        "0x740747e7e3a1e112",
        "11",
        "11000000000000000000",
    );

    assert_eq!(output.compact.as_deref(), Some("11"));
    assert_eq!(output.default, "11 PUSD\nAddress: 0x740747e7e3a1e112");
    assert_eq!(
        &output.value,
        &json!({
            "address": "0x740747e7e3a1e112",
            "balance": "11",
            "chain": "payy-dev",
            "native_symbol": "PUSD",
            "rpc_url": "http://127.0.0.1:8546",
            "wei": "11000000000000000000",
        })
    );
}
