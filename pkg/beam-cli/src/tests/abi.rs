use contracts::U256;
use serde_json::json;
use web3::ethabi::{StateMutability, Token, encode};

use crate::abi::{decode_output, parse_function, tokens_to_json};

#[test]
fn formats_signed_contract_outputs_as_signed_decimals() {
    let function = parse_function("inspect():(int256,uint256)", StateMutability::View)
        .expect("parse function");
    let encoded = encode(&[
        Token::Int(U256::MAX - U256::from(41u8)),
        Token::Uint(U256::from(7u8)),
    ]);

    let decoded = decode_output(&function, &encoded).expect("decode output");

    assert_eq!(tokens_to_json(&decoded), json!(["-42", "7"]));
}
