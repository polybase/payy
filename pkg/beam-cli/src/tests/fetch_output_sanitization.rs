use std::io::Cursor;

use contracts::{Address, Client, U256};
use serde_json::Value;

use crate::{
    chains::BeamChains,
    cli::FetchArgs,
    commands::fetch::{
        payment::{
            GasEstimate, PaymentAsset, PaymentAssetKind, PaymentChain, PreparedPayment,
            approve_payment_with,
        },
        protocol::{
            AmountValue, MppAuthChallenge, MppChallenge, MppPaymentRequest, MppProblem,
            PaymentChallenge, X402Challenge, X402Offer,
        },
    },
};

#[test]
fn x402_describe_sanitizes_human_facing_offer_fields() {
    let challenge = PaymentChallenge::X402(X402Challenge {
        offers: vec![X402Offer {
            amount: AmountValue::Atomic("1000\n\x1b[31m".to_string()),
            asset: "USDC\t\x1b[31m".to_string(),
            network: "eip155:8453\r\x1b[31m".to_string(),
            pay_to: "0xabc\x1b[31m".to_string(),
            raw: Value::Null,
            scheme: "exact\n\x1b[31m".to_string(),
        }],
        resource: None,
        version: 2,
    });

    assert_eq!(
        challenge.describe(),
        "Payment required via x402\nOffers: 1\n- 1000 ?[31m USDC ?[31m on eip155:8453 ?[31m to 0xabc?[31m (exact ?[31m)"
    );
}

#[test]
fn mpp_describe_sanitizes_human_facing_problem_fields() {
    let challenge = PaymentChallenge::Mpp(Box::new(MppChallenge {
        auth: Some(MppAuthChallenge {
            description: None,
            digest: None,
            expires: None,
            id: "challenge_123".to_string(),
            intent: "charge".to_string(),
            method: "tempo.charge\n\x1b[31m".to_string(),
            opaque: None,
            realm: "api.example.com".to_string(),
            request: "request".to_string(),
        }),
        problem: MppProblem {
            challenge_id: "challenge_123\n\x1b[31m".to_string(),
            detail: Some("Detail\t\x1b[31m".to_string()),
            title: Some("Title\r\x1b[31m".to_string()),
        },
        request: MppPaymentRequest {
            amount: AmountValue::Human("0.01\n\x1b[31m".to_string()),
            chain_id: Some(8453),
            currency: "USDC\t\x1b[31m".to_string(),
            description: Some("Invoice\n\x1b[31m".to_string()),
            recipient: "0x333\r\x1b[31m".to_string(),
        },
    }));

    assert_eq!(
        challenge.describe(),
        "Payment required via MPP\nChallenge: challenge_123 ?[31m\nTitle: Title ?[31m\nDetail: Detail ?[31m\nMethod: tempo.charge ?[31m 0.01 ?[31m USDC ?[31m 0x333 ?[31m"
    );
}

#[test]
fn confirmation_message_sanitizes_payment_details() {
    let mut payment = payment_fixture();
    payment.asset.label = "USDC\n\x1b[31m".to_string();
    payment.chain.display_name = "Base\t\x1b[31m".to_string();
    payment.chain.native_symbol = "ETH\r\x1b[31m".to_string();
    payment.description = Some("Invoice\n\x1b[31m".to_string());

    let confirmation = payment.confirmation_message("MPP");

    assert!(!confirmation.contains('\x1b'));
    assert!(confirmation.contains("Amount: 1 USDC ?[31m"));
    assert!(confirmation.contains("Network: Base ?[31m (8453)"));
    assert!(confirmation.contains("Estimated gas: 0.001 ETH ?[31m"));
    assert!(confirmation.contains("Description: Invoice ?[31m"));
}

#[test]
fn cross_chain_prompt_sanitizes_chain_summaries() {
    let mut payment = payment_fixture();
    payment.chain = payment_chain(8453, "Base\n\x1b[31m", "base");
    payment.selected_chain = Some(payment_chain(1, "Ethereum\t\x1b[31m", "ethereum"));

    let mut input = Cursor::new("yes\n");
    let mut output = Vec::new();

    approve_payment_with(
        &fetch_args(Some("2.0")),
        &payment,
        &BeamChains::default(),
        &mut input,
        &mut output,
    )
    .expect("approve sanitized prompt");

    let prompt = String::from_utf8(output).expect("prompt utf8");
    assert!(!prompt.contains('\x1b'));
    assert!(prompt.contains("Base ?[31m (8453)"));
    assert!(prompt.contains("Ethereum ?[31m (1)"));
}

fn fetch_args(max_fee: Option<&str>) -> FetchArgs {
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
        allowed_chains: Vec::new(),
        no_pay: false,
        dev: false,
    }
}

fn payment_fixture() -> PreparedPayment {
    PreparedPayment {
        accepted: Value::Null,
        amount: U256::from(1u64),
        amount_display: "1".to_string(),
        asset: PaymentAsset {
            decimals: 6,
            kind: PaymentAssetKind::Erc20(Address::from_low_u64_be(0xfeed)),
            label: "USDC".to_string(),
        },
        asset_id: "asset".to_string(),
        chain: payment_chain(8453, "Base", "base"),
        client: Client::new("http://localhost:8545", None),
        description: None,
        gas: GasEstimate {
            fee: U256::exp10(15),
            gas_limit: U256::from(21_000u64),
            gas_price: U256::from(1_000_000_000u64),
        },
        network: "eip155:8453".to_string(),
        payer: Address::from_low_u64_be(1),
        recipient: Address::from_low_u64_be(2),
        selected_chain: Some(payment_chain(8453, "Base", "base")),
        scheme: "exact".to_string(),
    }
}

fn payment_chain(chain_id: u64, display_name: &str, key: &str) -> PaymentChain {
    PaymentChain {
        aliases: Vec::new(),
        chain_id,
        display_name: display_name.to_string(),
        key: key.to_string(),
        native_symbol: "ETH".to_string(),
    }
}
