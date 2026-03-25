// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use serde_json::Value;
use web3::ethabi::{
    Event, EventParam, Function, Param, ParamType, RawLog, StateMutability, Token, decode, encode,
};

use crate::{
    abi::{
        encode_input, parse_function, read_param_type, token_to_json, tokenize_param,
        tokens_to_json,
    },
    error::{Error, Result},
    util::bytes::{decode_hex, hex_encode},
};

use super::abi_topic::indexed_topic_bytes;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedEvent {
    pub data: String,
    pub topics: Vec<String>,
}

pub fn abi_encode(signature: &str, args: &[String]) -> Result<String> {
    let function = parse_function(signature, StateMutability::Pure)?;
    let tokens = tokenize_params(&function.inputs, args)?;
    Ok(hex_encode(&encode(&tokens)))
}

pub fn abi_encode_event(signature: &str, args: &[String]) -> Result<EncodedEvent> {
    let event = parse_event(signature)?;
    let tokens = tokenize_event_inputs(&event, args)?;
    let mut topics = vec![hex_encode(event.signature().as_bytes())];
    let mut data_tokens = Vec::new();

    for (param, token) in event.inputs.iter().zip(tokens) {
        if param.indexed {
            topics.push(hex_encode(&indexed_topic_bytes(&token, &param.kind)?));
        } else {
            data_tokens.push(token);
        }
    }

    Ok(EncodedEvent {
        data: hex_encode(&encode(&data_tokens)),
        topics,
    })
}

pub fn calldata(signature: &str, args: &[String]) -> Result<String> {
    let function = parse_function(signature, StateMutability::Pure)?;
    Ok(hex_encode(&encode_input(&function, args)?))
}

pub fn decode_abi(signature: &str, data: &str, input: bool) -> Result<Vec<Value>> {
    let function = parse_function(signature, StateMutability::Pure)?;
    let bytes = decode_hex(data)?;
    let types = params_for_decode(&function, input);
    decode_values(&types, &bytes)
}

pub fn decode_calldata(signature: &str, data: &str) -> Result<Vec<Value>> {
    let function = parse_function(signature, StateMutability::Pure)?;
    let bytes = decode_hex(data)?;
    if bytes.len() < 4 {
        return Err(Error::SelectorMismatch {
            expected: hex_encode(&function.short_signature()),
            got: hex_encode(&bytes),
        });
    }

    let got = hex_encode(&bytes[..4]);
    let expected = hex_encode(&function.short_signature());
    if bytes[..4] != function.short_signature() {
        return Err(Error::SelectorMismatch { expected, got });
    }

    decode_values(&params_for_decode(&function, true), &bytes[4..])
}

pub fn decode_error(signature: &str, data: &str) -> Result<Vec<Value>> {
    let function = parse_function(signature, StateMutability::Pure)?;
    let bytes = decode_hex(data)?;
    if bytes.len() < 4 {
        return Err(Error::SelectorMismatch {
            expected: hex_encode(&function.short_signature()),
            got: hex_encode(&bytes),
        });
    }

    let got = hex_encode(&bytes[..4]);
    let expected = hex_encode(&function.short_signature());
    if bytes[..4] != function.short_signature() {
        return Err(Error::SelectorMismatch { expected, got });
    }

    decode_values(&params_for_decode(&function, true), &bytes[4..])
}

pub fn decode_event(
    signature: &str,
    data: &str,
    topics: &[String],
) -> Result<Vec<(String, Value)>> {
    let event = parse_event(signature)?;
    let indexed_count = event.inputs.iter().filter(|param| param.indexed).count();
    if indexed_count != topics.len() {
        return Err(Error::InvalidTopicCount {
            expected: indexed_count,
            got: topics.len(),
        });
    }

    let mut raw_topics = Vec::with_capacity(indexed_count + 1);
    raw_topics.push(event.signature());
    for topic in topics {
        let bytes = decode_hex(topic)?;
        if bytes.len() != 32 {
            return Err(Error::InvalidHexData {
                value: topic.to_string(),
            });
        }
        raw_topics.push(web3::ethabi::Hash::from_slice(&bytes));
    }

    let raw = RawLog {
        topics: raw_topics,
        data: decode_hex(data)?,
    };
    let decoded = event.parse_log(raw).context("decode beam event log")?;

    Ok(decoded
        .params
        .into_iter()
        .map(|param| (param.name, token_to_json(&param.value)))
        .collect())
}

pub fn decode_string(data: &str) -> Result<String> {
    let bytes = decode_hex(data)?;
    let values = decode_values(&[ParamType::String], &bytes)?;
    match values.as_slice() {
        [Value::String(value)] => Ok(value.clone()),
        _ => unreachable!("single string decode"),
    }
}

fn decode_values(types: &[ParamType], data: &[u8]) -> Result<Vec<Value>> {
    let tokens = decode(types, data).context("decode beam abi values")?;
    let Value::Array(values) = tokens_to_json(&tokens) else {
        unreachable!("token array");
    };
    Ok(values)
}

fn params_for_decode(function: &Function, input: bool) -> Vec<ParamType> {
    let params = if input {
        &function.inputs
    } else {
        &function.outputs
    };

    params.iter().map(|param| param.kind.clone()).collect()
}

fn parse_event(signature: &str) -> Result<Event> {
    let signature = signature.trim();
    let open = signature
        .find('(')
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: signature.to_string(),
        })?;
    let close = signature
        .rfind(')')
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: signature.to_string(),
        })?;
    let name = signature[..open].trim();
    if name.is_empty() || close < open {
        return Err(Error::InvalidFunctionSignature {
            signature: signature.to_string(),
        });
    }

    Ok(Event {
        name: name.to_string(),
        inputs: split_top_level(&signature[open + 1..close])?
            .into_iter()
            .enumerate()
            .map(|(index, item)| parse_event_param(index, &item, signature))
            .collect::<Result<Vec<_>>>()?,
        anonymous: false,
    })
}

fn parse_event_param(index: usize, value: &str, signature: &str) -> Result<EventParam> {
    let indexed = value.split_whitespace().any(|part| part == "indexed");
    let kind = value
        .split_whitespace()
        .find(|part| *part != "indexed")
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: value.to_string(),
        })?;

    Ok(EventParam {
        name: format!("arg{index}"),
        kind: read_param_type(kind, signature)?,
        indexed,
    })
}

fn split_top_level(list: &str) -> Result<Vec<String>> {
    let list = list.trim();
    if list.is_empty() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in list.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(list[start..index].trim().to_string());
                start = index + 1;
            }
            _ => {}
        }
    }

    items.push(list[start..].trim().to_string());
    Ok(items)
}

fn tokenize_event_inputs(event: &Event, args: &[String]) -> Result<Vec<Token>> {
    if event.inputs.len() != args.len() {
        return Err(Error::InvalidArgumentCount {
            expected: event.inputs.len(),
            got: args.len(),
        });
    }

    event
        .inputs
        .iter()
        .zip(args)
        .map(|(param, arg)| tokenize_param(&param.kind, arg))
        .collect()
}

fn tokenize_params(params: &[Param], args: &[String]) -> Result<Vec<Token>> {
    if params.len() != args.len() {
        return Err(Error::InvalidArgumentCount {
            expected: params.len(),
            got: args.len(),
        });
    }

    params
        .iter()
        .zip(args)
        .map(|(param, arg)| tokenize_param(&param.kind, arg))
        .collect()
}
