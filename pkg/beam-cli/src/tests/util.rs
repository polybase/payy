// lint-long-file-override allow-max-lines=300
use serde_json::json;
use web3::signing::keccak256;

use crate::{
    error::Error,
    runtime::parse_address,
    util::{abi, bytes, hash, numbers, rlp},
};

const ALICE: &str = "0x1111111111111111111111111111111111111111";
const BOB: &str = "0x2222222222222222222222222222222222222222";

#[test]
fn abi_encode_and_decode_calldata_round_trip() {
    let encoded = abi::abi_encode(
        "transfer(address,uint256)",
        &[ALICE.to_string(), "5".to_string()],
    )
    .expect("encode abi args");
    assert_eq!(
        encoded,
        "0x00000000000000000000000011111111111111111111111111111111111111110000000000000000000000000000000000000000000000000000000000000005"
    );

    let calldata = abi::calldata(
        "transfer(address,uint256)",
        &[ALICE.to_string(), "5".to_string()],
    )
    .expect("encode calldata");
    assert_eq!(
        calldata,
        "0xa9059cbb00000000000000000000000011111111111111111111111111111111111111110000000000000000000000000000000000000000000000000000000000000005"
    );

    let decoded =
        abi::decode_calldata("transfer(address,uint256)", &calldata).expect("decode calldata");
    assert_eq!(decoded, vec![json!(ALICE), json!("5")]);
}

#[test]
fn abi_event_encode_and_decode_round_trip() {
    let encoded = abi::abi_encode_event(
        "Transfer(address indexed,address indexed,uint256)",
        &[ALICE.to_string(), BOB.to_string(), "5".to_string()],
    )
    .expect("encode event");
    assert_eq!(encoded.topics.len(), 3);
    assert_eq!(
        encoded.topics[0],
        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
    );

    let decoded = abi::decode_event(
        "Transfer(address indexed,address indexed,uint256)",
        &encoded.data,
        &encoded.topics[1..],
    )
    .expect("decode event");
    assert_eq!(
        decoded,
        vec![
            ("arg0".to_string(), json!(ALICE)),
            ("arg1".to_string(), json!(BOB)),
            ("arg2".to_string(), json!("5")),
        ]
    );
}

#[test]
fn abi_event_encode_hashes_indexed_string_topics_like_solidity() {
    let encoded =
        abi::abi_encode_event("E(string indexed)", &["abc".to_string()]).expect("encode event");

    assert_eq!(encoded.data, "0x");
    assert_eq!(encoded.topics[1], bytes::hex_encode(&keccak256(b"abc")));
}

#[test]
fn abi_event_encode_hashes_indexed_bytes_topics_like_solidity() {
    let encoded =
        abi::abi_encode_event("E(bytes indexed)", &["0x616263".to_string()]).expect("encode event");

    assert_eq!(encoded.data, "0x");
    assert_eq!(encoded.topics[1], bytes::hex_encode(&keccak256(b"abc")));
}

#[test]
fn calldata_rejects_invalid_numeric_arguments_with_invalid_number() {
    let err = abi::calldata("transfer(uint256)", &["not-a-number".to_string()])
        .expect_err("reject invalid uint calldata argument");

    assert!(matches!(err, Error::InvalidNumber { value } if value == "not-a-number"));
}

#[test]
fn abi_encode_rejects_invalid_numeric_arguments_with_invalid_number() {
    let err = abi::abi_encode("transfer(uint256)", &["not-a-number".to_string()])
        .expect_err("reject invalid uint abi argument");

    assert!(matches!(err, Error::InvalidNumber { value } if value == "not-a-number"));
}

#[test]
fn abi_encode_rejects_invalid_bool_arguments_with_invalid_abi_argument() {
    let err = abi::abi_encode("setFlag(bool)", &["maybe".to_string()])
        .expect_err("reject invalid bool abi argument");

    assert!(
        matches!(err, Error::InvalidAbiArgument { kind, value } if kind == "bool" && value == "maybe")
    );
}

#[test]
fn calldata_rejects_malformed_function_signatures() {
    let err = abi::calldata("transfer(uint2x)", &["5".to_string()])
        .expect_err("reject malformed function signature");

    assert!(
        matches!(err, Error::InvalidFunctionSignature { signature } if signature == "transfer(uint2x)")
    );
}

#[test]
fn abi_encode_event_rejects_malformed_event_signatures() {
    let err = abi::abi_encode_event("Transfer(uint2x indexed)", &["5".to_string()])
        .expect_err("reject malformed event signature");

    assert!(
        matches!(err, Error::InvalidFunctionSignature { signature } if signature == "Transfer(uint2x indexed)")
    );
}

#[test]
fn mapping_index_surfaces_key_type_and_value_errors() {
    let err =
        hash::mapping_index("uint2x", "5", "1").expect_err("reject malformed mapping key type");

    assert!(matches!(err, Error::InvalidFunctionSignature { signature } if signature == "uint2x"));

    let err = hash::mapping_index("uint256", "not-a-number", "1")
        .expect_err("reject invalid mapping key value");

    assert!(matches!(err, Error::InvalidNumber { value } if value == "not-a-number"));
}

#[test]
fn numeric_utilities_match_cast_style() {
    assert_eq!(
        numbers::format_units_value("1234000", "6").expect("format units"),
        "1.234000"
    );
    assert_eq!(
        numbers::parse_units_value("1.234000", "6").expect("parse units"),
        "1234000"
    );
    assert_eq!(
        numbers::from_wei("1000000000", Some("gwei")).expect("from wei"),
        "1.000000000"
    );
    assert_eq!(
        numbers::to_unit("1gwei", Some("ether")).expect("to unit"),
        "0.000000001000000000"
    );
    assert_eq!(
        numbers::to_wei("1", Some("gwei")).expect("to wei"),
        "1000000000"
    );
}

#[test]
fn numeric_utilities_reject_unsupported_decimal_precision() {
    let err = numbers::parse_units_value("1", "1000").expect_err("reject unsupported decimals");

    assert!(matches!(
        err,
        Error::UnsupportedDecimals {
            decimals: 1000,
            max: 77,
        }
    ));
}

#[test]
fn base_and_shift_utilities_work() {
    assert_eq!(
        numbers::to_base("0xff", None, "2").expect("to base"),
        "0b11111111"
    );
    assert_eq!(numbers::to_hex("255", None).expect("to hex"), "0xff");
    assert_eq!(numbers::to_dec("0xff", None).expect("to dec"), "255");
    assert_eq!(
        numbers::shift_left("0xff", "8", None, None).expect("shift left"),
        "0xff00"
    );
    assert_eq!(
        numbers::shift_right("0xff00", "8", None, None).expect("shift right"),
        "0xff"
    );
}

#[test]
fn integer_bounds_and_word_encodings_work() {
    assert_eq!(numbers::max_int(Some("int8")).expect("max int"), "127");
    assert_eq!(numbers::min_int(Some("int8")).expect("min int"), "-128");
    assert_eq!(numbers::max_uint(Some("uint8")).expect("max uint"), "255");
    assert_eq!(
        numbers::to_uint256("1").expect("to uint256"),
        "0x0000000000000000000000000000000000000000000000000000000000000001"
    );
    assert_eq!(
        numbers::to_int256("-1").expect("to int256"),
        "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
}

#[test]
fn bytes_and_hex_utilities_work() {
    let bytes32 = bytes::format_bytes32_string("hi").expect("format bytes32 string");
    assert_eq!(
        bytes32,
        "0x6869000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(
        bytes::parse_bytes32_string(&bytes32).expect("parse bytes32 string"),
        "hi"
    );
    assert_eq!(
        bytes::normalize_hexdata("0xAB:cd").expect("normalize hexdata"),
        "0xabcd"
    );
    assert_eq!(bytes::utf8_to_hex("hi"), "0x6869");
    assert_eq!(bytes::hex_to_utf8("0x6869").expect("to utf8"), "hi");
}

#[test]
fn rlp_round_trip_works() {
    let encoded = rlp::to_rlp("[\"0x01\",\"0x0203\"]").expect("encode rlp");
    assert_eq!(encoded, "0xc401820203");
    assert_eq!(
        rlp::from_rlp(&encoded, false).expect("decode rlp"),
        json!(["0x01", "0x0203"])
    );
}

#[test]
fn hashing_and_storage_helpers_match_known_vectors() {
    assert_eq!(hash::selector("transfer(address,uint256)"), "0xa9059cbb");
    assert_eq!(
        hash::mapping_index("address", ALICE, "1").expect("mapping index"),
        "0x8eec1c9afb183a84aac7003cf8e730bfb6385f6e43761d6425fba4265de3a9eb"
    );
    assert_eq!(
        hash::erc7201_index("my.namespace"),
        "0xb169b0b3596cb35d7e5b5cdf8d022e0104e5084d855821f476f9d23b51360a00"
    );
}

#[test]
fn contract_address_helpers_match_known_vectors() {
    assert_eq!(
        hash::compute_address(
            Some("0x0000000000000000000000000000000000000000"),
            Some("1"),
            None,
            None,
            None
        )
        .expect("compute address"),
        "0x5a443704dd4B594B382c22a083e2BD3090A6feF3"
    );
    assert_eq!(
        hash::create2_address(
            Some("0x0000000000000000000000000000000000000000"),
            Some("0x0000000000000000000000000000000000000000000000000000000000000000"),
            Some("0x00"),
            None,
        )
        .expect("create2 address"),
        "0x4D1A2e2bB4F88F0250f26Ffff098B0b30B26BF38"
    );
}

#[test]
fn checksummed_address_supports_chain_id() {
    let address =
        parse_address("0x52908400098527886e0f7030069857d2e4169ee7").expect("parse address");
    assert_eq!(
        hash::checksum_address(address, Some(30)),
        "0x52908400098527886E0F7030069857D2E4169ee7"
    );
}
