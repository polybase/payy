use contracts::{Address, Client, U256};
use serde_json::Value;
use web3::ethabi::StateMutability;

use super::evm::{RpcScenario, rpc_methods, spawn_rpc_server};
use crate::{
    abi::parse_function,
    evm::{FunctionCall, estimate_function_gas},
};

#[tokio::test]
async fn function_gas_estimation_encodes_call_without_submission() {
    let (rpc_url, calls, server) = spawn_rpc_server(RpcScenario::Confirmed).await;
    let client = Client::try_new(&rpc_url, None).expect("create client");
    let from = Address::from_low_u64_be(0x1234);
    let contract = Address::from_low_u64_be(0xfeed);
    let function = parse_function("transfer(address,uint256)", StateMutability::NonPayable)
        .expect("parse function");
    let args = vec![
        format!("{:#x}", Address::from_low_u64_be(0xbeef)),
        U256::from(123u64).to_string(),
    ];

    let gas = estimate_function_gas(
        &client,
        from,
        FunctionCall {
            args: &args,
            contract,
            function: &function,
            value: U256::zero(),
        },
    )
    .await
    .expect("estimate function gas");
    server.abort();

    assert_eq!(gas.gas_limit, U256::from(36_000u64));
    assert_eq!(gas.gas_price_for_display(), U256::from(3_000_000_000u64));

    let calls = calls.lock().expect("rpc calls").clone();
    assert_eq!(
        rpc_methods(&calls),
        vec!["eth_estimateGas", "eth_chainId", "eth_feeHistory"]
    );
    let estimate = &calls[0]["params"][0];
    assert_eq!(estimate["from"], Value::String(format!("{from:#x}")));
    assert_eq!(estimate["to"], Value::String(format!("{contract:#x}")));
    assert_eq!(estimate["value"], Value::String("0x0".to_string()));
    assert!(
        estimate["data"]
            .as_str()
            .expect("encoded data")
            .starts_with("0xa9059cbb")
    );
}
