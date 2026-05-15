// lint-long-file-override allow-max-lines=300
use std::io::Cursor;

use contracts::{Address, Client, U256};
use serde_json::Value;

use crate::{
    chains::BeamChains,
    cli::FetchArgs,
    commands::fetch::payment::{
        GasEstimate, PaymentAsset, PaymentAssetKind, PaymentChain, PreparedPayment,
        approve_payment, approve_payment_with,
    },
    error::Error,
    evm::parse_units,
};

#[test]
fn approve_payment_rejects_native_total_above_max_fee() {
    let payment = payment_fixture(
        PaymentAsset {
            decimals: 18,
            kind: PaymentAssetKind::Native,
            label: "ETH".to_string(),
        },
        parse_units("0.95", 18).expect("native amount"),
        parse_units("0.10", 18).expect("gas fee"),
    );

    let err = approve_payment(
        &fetch_args(Some("1.0"), &[]),
        &payment,
        &BeamChains::default(),
    )
    .expect_err("reject max fee");

    assert!(matches!(err, Error::FetchPaymentExceedsMaxFee));
}

#[test]
fn approve_payment_accepts_native_total_within_max_fee() {
    let payment = payment_fixture(
        PaymentAsset {
            decimals: 18,
            kind: PaymentAssetKind::Native,
            label: "ETH".to_string(),
        },
        parse_units("0.90", 18).expect("native amount"),
        parse_units("0.10", 18).expect("gas fee"),
    );

    approve_payment(
        &fetch_args(Some("1.0"), &[]),
        &payment,
        &BeamChains::default(),
    )
    .expect("approve max fee");
}

#[test]
fn approve_payment_rejects_token_payment_when_gas_exceeds_max_fee() {
    let payment = payment_fixture(
        PaymentAsset {
            decimals: 6,
            kind: PaymentAssetKind::Erc20(Address::from_low_u64_be(0xfeed)),
            label: "USDC".to_string(),
        },
        parse_units("0.01", 6).expect("token amount"),
        parse_units("0.02", 18).expect("gas fee"),
    );

    let err = approve_payment(
        &fetch_args(Some("0.01"), &[]),
        &payment,
        &BeamChains::default(),
    )
    .expect_err("reject max fee");

    assert!(matches!(err, Error::FetchPaymentExceedsMaxFee));
}

#[test]
fn approve_payment_accepts_token_payment_when_amount_and_gas_fit_cap() {
    let payment = payment_fixture(
        PaymentAsset {
            decimals: 6,
            kind: PaymentAssetKind::Erc20(Address::from_low_u64_be(0xfeed)),
            label: "USDC".to_string(),
        },
        parse_units("0.01", 6).expect("token amount"),
        parse_units("0.001", 18).expect("gas fee"),
    );

    approve_payment(
        &fetch_args(Some("0.01"), &[]),
        &payment,
        &BeamChains::default(),
    )
    .expect("approve max fee");
}

#[test]
fn approve_payment_rejects_chain_outside_allowlist() {
    let mut payment = payment_fixture(
        PaymentAsset {
            decimals: 18,
            kind: PaymentAssetKind::Native,
            label: "ETH".to_string(),
        },
        parse_units("0.10", 18).expect("native amount"),
        parse_units("0.01", 18).expect("gas fee"),
    );
    payment.chain = payment_chain(1, "Ethereum", "ethereum", &["mainnet"]);
    payment.selected_chain = Some(payment_chain(8453, "Base", "base", &[]));

    let err = approve_payment(
        &fetch_args(Some("1.0"), &["base"]),
        &payment,
        &BeamChains::default(),
    )
    .expect_err("reject disallowed chain");

    assert!(matches!(err, Error::FetchPaymentChainNotAllowed { .. }));
}

#[test]
fn approve_payment_accepts_chain_inside_allowlist() {
    let mut payment = payment_fixture(
        PaymentAsset {
            decimals: 18,
            kind: PaymentAssetKind::Native,
            label: "ETH".to_string(),
        },
        parse_units("0.10", 18).expect("native amount"),
        parse_units("0.01", 18).expect("gas fee"),
    );
    payment.chain = payment_chain(1, "Ethereum", "ethereum", &["mainnet"]);
    payment.selected_chain = Some(payment_chain(8453, "Base", "base", &[]));

    approve_payment(
        &fetch_args(Some("1.0"), &["ethereum"]),
        &payment,
        &BeamChains::default(),
    )
    .expect("approve allowed chain");
}

#[test]
fn approve_payment_accepts_chain_selector_with_find_chain_normalization() {
    let mut payment = payment_fixture(
        PaymentAsset {
            decimals: 18,
            kind: PaymentAssetKind::Native,
            label: "PUSD".to_string(),
        },
        parse_units("0.10", 18).expect("native amount"),
        parse_units("0.01", 18).expect("gas fee"),
    );
    payment.chain = payment_chain(7297, "Payy Dev", "payy-dev", &["payydev"]);
    payment.selected_chain = Some(payment_chain(8453, "Base", "base", &[]));

    approve_payment(
        &fetch_args(Some("1.0"), &["payy_dev"]),
        &payment,
        &BeamChains::default(),
    )
    .expect("approve normalized selector");
}

#[test]
fn approve_payment_prompts_before_accepting_cross_chain_request() {
    let mut payment = payment_fixture(
        PaymentAsset {
            decimals: 18,
            kind: PaymentAssetKind::Native,
            label: "ETH".to_string(),
        },
        parse_units("0.10", 18).expect("native amount"),
        parse_units("0.01", 18).expect("gas fee"),
    );
    payment.chain = payment_chain(1, "Ethereum", "ethereum", &["mainnet"]);
    payment.selected_chain = Some(payment_chain(8453, "Base", "base", &[]));

    let mut input = Cursor::new("yes\n");
    let mut output = Vec::new();

    approve_payment_with(
        &fetch_args(Some("1.0"), &[]),
        &payment,
        &BeamChains::default(),
        &mut input,
        &mut output,
    )
    .expect("approve prompted chain");

    let prompt = String::from_utf8(output).expect("prompt utf8");
    assert!(prompt.contains("Ethereum (1)"));
    assert!(prompt.contains("Base (8453)"));
}

fn fetch_args(max_fee: Option<&str>, allowed_chains: &[&str]) -> FetchArgs {
    FetchArgs {
        url: "https://api.example.com/paid".to_string(),
        method: Some("GET".to_string()),
        headers: Vec::new(),
        data: None,
        data_file: None,
        output_path: None,
        verbose: false,
        follow_redirects: false,
        max_redirects: 10,
        connect_timeout: None,
        timeout: None,
        max_fee: max_fee.map(ToString::to_string),
        allowed_chains: allowed_chains.iter().map(ToString::to_string).collect(),
        no_pay: false,
        dev: false,
        private_payment: false,
    }
}

fn payment_fixture(asset: PaymentAsset, amount: U256, gas_fee: U256) -> PreparedPayment {
    PreparedPayment {
        accepted: Value::Null,
        amount,
        amount_display: "0".to_string(),
        asset,
        asset_id: "asset".to_string(),
        chain: payment_chain(8453, "Base", "base", &["base-mainnet"]),
        client: Client::new("http://localhost:8545", None),
        description: None,
        gas: GasEstimate {
            fee: gas_fee,
            gas_limit: U256::from(21_000u64),
            gas_price: U256::from(1u64),
        },
        network: "eip155:8453".to_string(),
        payer: Address::from_low_u64_be(1),
        recipient: Address::from_low_u64_be(2),
        private_recipient: None,
        selected_chain: Some(payment_chain(8453, "Base", "base", &["base-mainnet"])),
        scheme: "exact".to_string(),
    }
}

fn payment_chain(chain_id: u64, display_name: &str, key: &str, aliases: &[&str]) -> PaymentChain {
    PaymentChain {
        aliases: aliases.iter().map(ToString::to_string).collect(),
        chain_id,
        display_name: display_name.to_string(),
        key: key.to_string(),
        native_symbol: "ETH".to_string(),
        privacy: None,
    }
}
