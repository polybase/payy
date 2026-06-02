use contracts::U256;
use serde_json::json;

use super::{GasOutputConfig, render_gas_output};
use crate::evm::TransactionGas;

#[test]
fn render_gas_output_includes_fee_details() {
    let output = render_gas_output(GasOutputConfig {
        chain_key: "base",
        default_summary: "Estimated gas".to_string(),
        extra: json!({ "kind": "transfer" }),
        gas: TransactionGas {
            gas_limit: U256::from(21_000u64),
            gas_price: U256::from(1_000_000_000u64),
        },
        native_symbol: "ETH",
    });

    assert!(output.default.contains("Estimated fee: 0.000021 ETH"));
    assert_eq!(output.value["estimated_fee"], "0.000021");
    assert_eq!(output.value["estimated_fee_wei"], "21000000000000");
    assert_eq!(output.value["gas_limit"], "21000");
    assert_eq!(output.value["gas_price"], "1000000000");
    assert_eq!(output.value["kind"], "transfer");
}
