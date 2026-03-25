use web3::ethabi::{ParamType, Token, encode};

use crate::error::{Error, Result};

pub(super) fn indexed_topic_bytes(token: &Token, kind: &ParamType) -> Result<[u8; 32]> {
    if !token.type_check(kind) {
        return Err(Error::InvalidFunctionSignature {
            signature: format!("{kind:?}"),
        });
    }

    match (token, kind) {
        (Token::Bytes(bytes), ParamType::Bytes) => Ok(web3::signing::keccak256(bytes)),
        (Token::String(value), ParamType::String) => Ok(web3::signing::keccak256(value.as_bytes())),
        (
            _,
            ParamType::Address
            | ParamType::Bool
            | ParamType::FixedBytes(_)
            | ParamType::Int(_)
            | ParamType::Uint(_),
        ) => Ok(topic_word(token)),
        _ => {
            let mut preimage = Vec::new();
            indexed_topic_preimage(token, kind, &mut preimage)?;
            Ok(web3::signing::keccak256(&preimage))
        }
    }
}

// Indexed event topics use Solidity's event-specific in-place encoding instead
// of standard ABI head/tail encoding for complex values.
fn indexed_topic_preimage(token: &Token, kind: &ParamType, out: &mut Vec<u8>) -> Result<()> {
    match (token, kind) {
        (
            _,
            ParamType::Address
            | ParamType::Bool
            | ParamType::FixedBytes(_)
            | ParamType::Int(_)
            | ParamType::Uint(_),
        ) => {
            out.extend_from_slice(&topic_word(token));
        }
        (Token::Bytes(bytes), ParamType::Bytes) => encode_padded_bytes(bytes, out),
        (Token::String(value), ParamType::String) => encode_padded_bytes(value.as_bytes(), out),
        (Token::Array(values), ParamType::Array(item_kind))
        | (Token::FixedArray(values), ParamType::FixedArray(item_kind, _)) => {
            for value in values {
                indexed_topic_preimage(value, item_kind, out)?;
            }
        }
        (Token::Tuple(values), ParamType::Tuple(item_kinds)) => {
            for (value, item_kind) in values.iter().zip(item_kinds) {
                indexed_topic_preimage(value, item_kind, out)?;
            }
        }
        _ => {
            return Err(Error::InvalidFunctionSignature {
                signature: format!("{kind:?}"),
            });
        }
    }

    Ok(())
}

fn encode_padded_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    let padding = match bytes.len() % 32 {
        0 if bytes.is_empty() => 32,
        0 => 0,
        remainder => 32 - remainder,
    };

    out.reserve(bytes.len() + padding);
    out.extend_from_slice(bytes);
    out.resize(out.len() + padding, 0);
}

fn topic_word(token: &Token) -> [u8; 32] {
    let encoded = encode(std::slice::from_ref(token));
    let mut topic = [0u8; 32];
    topic.copy_from_slice(&encoded);
    topic
}
