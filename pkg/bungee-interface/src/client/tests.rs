// lint-long-file-override allow-max-lines=300
use super::{
    BungeeStatusCode, Error, GetQuoteInput, GetQuoteOutput, GetStatusInput, GetStatusOutput,
    GetTokenListOutput, StatusEntry, TokenMetadata,
};
use contextful::ResultContextExt;
use contracts::{Address, U256};
use rpc::error::{ErrorOutput, HTTPError};
use serde_json::json;
use std::collections::BTreeMap;

fn sample_address(value: u64) -> Address {
    Address::from_low_u64_be(value)
}

fn assert_error_round_trip(name: &str, error: Error) {
    let http_error = HTTPError::from(error.clone());
    let output = ErrorOutput::from(http_error);
    insta::assert_json_snapshot!(name, output);

    let original = error.clone();
    let round_trip = Error::try_from(HTTPError::from(error)).expect("round trip error");
    match round_trip {
        Error::UnsupportedSourceChainId { chain_id } => {
            assert!(
                matches!(original, Error::UnsupportedSourceChainId { chain_id: other } if other == chain_id)
            );
        }
        Error::NoRoute => assert!(matches!(original, Error::NoRoute)),
        Error::MissingStatusIdentifier => {
            assert!(matches!(original, Error::MissingStatusIdentifier));
        }
        Error::InputAmountTooLow { usd_amount } => {
            assert!(
                matches!(original, Error::InputAmountTooLow { usd_amount: other } if (other - usd_amount).abs() < f64::EPSILON)
            );
        }
        Error::OutputAmountTooLow { usd_amount } => {
            assert!(
                matches!(original, Error::OutputAmountTooLow { usd_amount: other } if (other - usd_amount).abs() < f64::EPSILON)
            );
        }
        Error::Internal(_) => panic!("internal errors are encode-only"),
    }
}

#[test]
fn domain_error_wire_formats_are_stable() {
    assert_error_round_trip(
        "unsupported_source_chain_id",
        Error::UnsupportedSourceChainId { chain_id: 8453 },
    );
    assert_error_round_trip("no_route", Error::NoRoute);
    assert_error_round_trip("missing_status_identifier", Error::MissingStatusIdentifier);
    assert_error_round_trip(
        "input_amount_too_low",
        Error::InputAmountTooLow { usd_amount: 0.01 },
    );
    assert_error_round_trip(
        "output_amount_too_low",
        Error::OutputAmountTooLow { usd_amount: 0.02 },
    );
}

#[test]
fn internal_error_wire_format_is_encode_only() {
    let internal = Err::<(), _>(std::io::Error::other("boom"))
        .context("build internal error")
        .unwrap_err();
    let output = ErrorOutput::from(HTTPError::from(Error::from(internal)));
    insta::assert_json_snapshot!("internal_error", output);
}

#[test]
fn quote_input_round_trips_as_json() {
    let input = GetQuoteInput {
        source_chain_id: 10,
        destination_chain_id: 42161,
        input_token: sample_address(1),
        output_token: sample_address(2),
        input_amount: U256::from(123u64),
        receiver_address: sample_address(3),
        user_address: sample_address(4),
    };
    let value = serde_json::to_value(&input).expect("serialize quote input");
    insta::assert_json_snapshot!("quote_input", value);
    let decoded = serde_json::from_value::<GetQuoteInput>(value).expect("deserialize quote input");
    assert_eq!(decoded.source_chain_id, input.source_chain_id);
    assert_eq!(decoded.destination_chain_id, input.destination_chain_id);
    assert_eq!(decoded.input_token, input.input_token);
    assert_eq!(decoded.output_token, input.output_token);
    assert_eq!(decoded.input_amount, input.input_amount);
    assert_eq!(decoded.receiver_address, input.receiver_address);
    assert_eq!(decoded.user_address, input.user_address);
}

#[test]
fn quote_output_round_trips_as_json() {
    let output = GetQuoteOutput {
        output_amount: U256::from(456u64),
        min_output_amount: Some(U256::from(400u64)),
        tx_to: sample_address(5),
        tx_value: U256::from(789u64),
        tx_data: vec![0xde, 0xad, 0xbe, 0xef],
        approval_spender: Some(sample_address(6)),
        approval_amount: Some(U256::from(999u64)),
        quote_id: Some("qid-1".to_owned()),
        request_hash: Some("rh-1".to_owned()),
    };
    let value = serde_json::to_value(&output).expect("serialize quote output");
    insta::assert_json_snapshot!("quote_output", value);
    let decoded =
        serde_json::from_value::<GetQuoteOutput>(value).expect("deserialize quote output");
    assert_eq!(decoded, output);
}

#[test]
fn quote_output_deserializes_without_min_output_amount() {
    let value = json!({
        "output_amount": "0x1c8",
        "tx_to": sample_address(5),
        "tx_value": "0x315",
        "tx_data": "0xdeadbeef",
        "approval_spender": sample_address(6),
        "approval_amount": "0x3e7",
        "quote_id": "qid-1",
        "request_hash": "rh-1"
    });
    let decoded =
        serde_json::from_value::<GetQuoteOutput>(value).expect("deserialize legacy quote output");

    assert_eq!(decoded.min_output_amount, None);
}

#[test]
fn token_list_output_round_trips_as_json() {
    let output = GetTokenListOutput {
        tokens: BTreeMap::from([(
            137u128,
            vec![TokenMetadata {
                address: sample_address(7),
                name: "USD Coin".to_owned(),
                symbol: "USDC".to_owned(),
                decimals: 6,
                logo_uri: Some("https://example.com/usdc.png".to_owned()),
            }],
        )]),
    };
    let value = serde_json::to_value(&output).expect("serialize token list");
    insta::assert_json_snapshot!("token_list_output", value);
    let decoded =
        serde_json::from_value::<GetTokenListOutput>(value).expect("deserialize token list");
    assert_eq!(decoded, output);
}

#[test]
fn status_output_round_trips_as_json() {
    let output = GetStatusOutput {
        statuses: vec![StatusEntry {
            code: BungeeStatusCode::Fulfilled,
            label: Some("FULFILLED".to_owned()),
            destination_tx_hash: Some("0xabc".to_owned()),
        }],
    };
    let value = serde_json::to_value(&output).expect("serialize status output");
    insta::assert_json_snapshot!("status_output", value);
    let decoded =
        serde_json::from_value::<GetStatusOutput>(value).expect("deserialize status output");
    assert_eq!(decoded, output);
}

#[test]
fn bungee_status_code_variants_round_trip() {
    for (name, code) in [
        ("status_code_pending", BungeeStatusCode::Pending),
        ("status_code_assigned", BungeeStatusCode::Assigned),
        ("status_code_extracted", BungeeStatusCode::Extracted),
        ("status_code_fulfilled", BungeeStatusCode::Fulfilled),
        ("status_code_settled", BungeeStatusCode::Settled),
        ("status_code_expired", BungeeStatusCode::Expired),
        ("status_code_cancelled", BungeeStatusCode::Cancelled),
        ("status_code_refunded", BungeeStatusCode::Refunded),
        ("status_code_unknown", BungeeStatusCode::Unknown(99)),
    ] {
        let value = serde_json::to_value(code).expect("serialize status code");
        insta::assert_json_snapshot!(name, value);
        let decoded =
            serde_json::from_value::<BungeeStatusCode>(value).expect("deserialize status code");
        assert_eq!(decoded, code);
    }
}

#[test]
fn status_input_query_round_trip_uses_wire_keys() {
    for (name, input, expected_pairs) in [
        (
            "status_input_request_hash",
            GetStatusInput::from_request_hash("0xrequest"),
            json!([["requestHash", "0xrequest"]]),
        ),
        (
            "status_input_tx_hash",
            GetStatusInput::from_tx_hash("0xtx"),
            json!([["txHash", "0xtx"]]),
        ),
        (
            "status_input_id",
            GetStatusInput::from_id("permit-id"),
            json!([["id", "permit-id"]]),
        ),
    ] {
        let pairs = input.to_query_pairs().expect("query pairs");
        insta::assert_json_snapshot!(name, pairs);
        assert_eq!(
            serde_json::to_value(&pairs).expect("pairs json"),
            expected_pairs
        );

        let encoded = serde_urlencoded::to_string(&input).expect("encode query");
        let decoded = serde_urlencoded::from_str::<GetStatusInput>(&encoded).expect("decode query");
        assert_eq!(decoded, input);
    }
}
