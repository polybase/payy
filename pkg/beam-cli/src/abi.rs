// lint-long-file-override allow-max-lines=260
use contextful::ResultContextExt;
use serde_json::{Value, json};
use web3::ethabi::{
    Function, Param, ParamType, StateMutability, Token,
    ethereum_types::U256,
    param_type::Reader,
    token::{LenientTokenizer, Tokenizer},
};

use crate::error::{Error, Result};

pub fn parse_function(signature: &str, state_mutability: StateMutability) -> Result<Function> {
    let signature = signature.trim();
    let (input_signature, output_signature) = split_signature(signature)?;
    let open = input_signature
        .find('(')
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: signature.to_string(),
        })?;
    let name = input_signature[..open].trim();

    if name.is_empty() {
        return Err(Error::InvalidFunctionSignature {
            signature: signature.to_string(),
        });
    }

    let inputs = param_list(
        &input_signature[open + 1..input_signature.len() - 1],
        "arg",
        signature,
    )?;
    let outputs = match output_signature {
        Some(output_signature) => param_list(
            &output_signature[1..output_signature.len() - 1],
            "out",
            signature,
        )?,
        None => Vec::new(),
    };

    #[allow(deprecated)]
    let function = Function {
        name: name.to_string(),
        inputs,
        outputs,
        constant: None,
        state_mutability,
    };

    Ok(function)
}

pub fn encode_input(function: &Function, args: &[String]) -> Result<Vec<u8>> {
    let tokens = tokenize_args(function, args)?;
    let data = function
        .encode_input(&tokens)
        .context("encode beam abi input")?;
    Ok(data)
}

pub fn decode_output(function: &Function, data: &[u8]) -> Result<Vec<Token>> {
    if function.outputs.is_empty() {
        return Ok(Vec::new());
    }

    let tokens = function
        .decode_output(data)
        .context("decode beam abi output")?;
    Ok(tokens)
}

pub fn tokens_to_json(tokens: &[Token]) -> Value {
    Value::Array(tokens.iter().map(token_to_json).collect())
}

fn split_signature(signature: &str) -> Result<(String, Option<String>)> {
    let input_close = input_close_index(signature)?;
    let input = signature[..=input_close].to_string();
    let rest = signature[input_close + 1..].trim();

    if rest.is_empty() {
        return Ok((input, None));
    }

    let rest = rest.strip_prefix(':').unwrap_or(rest).trim();
    if rest.starts_with('(') && rest.ends_with(')') {
        return Ok((input, Some(rest.to_string())));
    }

    Err(Error::InvalidFunctionSignature {
        signature: signature.to_string(),
    })
}

fn input_close_index(signature: &str) -> Result<usize> {
    let open = signature
        .find('(')
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: signature.to_string(),
        })?;
    let mut depth = 0usize;

    for (index, ch) in signature.char_indices().skip(open) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }

    Err(Error::InvalidFunctionSignature {
        signature: signature.to_string(),
    })
}

fn param_list(list: &str, prefix: &str, signature: &str) -> Result<Vec<Param>> {
    let types = if list.trim().is_empty() {
        Vec::new()
    } else {
        tuple_items(list, signature)?
    };

    Ok(types
        .into_iter()
        .enumerate()
        .map(|(index, kind)| Param {
            name: format!("{prefix}{index}"),
            kind,
            internal_type: None,
        })
        .collect())
}

fn tokenize_args(function: &Function, args: &[String]) -> Result<Vec<Token>> {
    if function.inputs.len() != args.len() {
        return Err(Error::InvalidArgumentCount {
            expected: function.inputs.len(),
            got: args.len(),
        });
    }

    function
        .inputs
        .iter()
        .zip(args)
        .map(|(param, arg)| tokenize_param(&param.kind, arg))
        .collect()
}

fn tuple_items(list: &str, signature: &str) -> Result<Vec<ParamType>> {
    let type_string = format!("({})", list.replace(' ', ""));
    let tuple = read_param_type(&type_string, signature)?;

    match tuple {
        ParamType::Tuple(items) => Ok(items),
        _ => unreachable!(),
    }
}

pub(crate) fn read_param_type(kind: &str, signature: &str) -> Result<ParamType> {
    Reader::read(kind).map_err(|_| Error::InvalidFunctionSignature {
        signature: signature.to_string(),
    })
}

pub(crate) fn tokenize_param(kind: &ParamType, value: &str) -> Result<Token> {
    LenientTokenizer::tokenize(kind, value).map_err(|_| invalid_abi_argument(kind, value))
}

fn invalid_abi_argument(kind: &ParamType, value: &str) -> Error {
    match kind {
        ParamType::Address => Error::InvalidAddress {
            value: value.to_string(),
        },
        ParamType::Uint(_) | ParamType::Int(_) => Error::InvalidNumber {
            value: value.to_string(),
        },
        ParamType::Bytes | ParamType::FixedBytes(_) => Error::InvalidHexData {
            value: value.to_string(),
        },
        ParamType::Bool => Error::InvalidAbiArgument {
            kind: "bool".to_string(),
            value: value.to_string(),
        },
        ParamType::String => Error::InvalidAbiArgument {
            kind: "string".to_string(),
            value: value.to_string(),
        },
        ParamType::Array(_) | ParamType::FixedArray(_, _) => Error::InvalidAbiArgument {
            kind: "array".to_string(),
            value: value.to_string(),
        },
        ParamType::Tuple(_) => Error::InvalidAbiArgument {
            kind: "tuple".to_string(),
            value: value.to_string(),
        },
    }
}

pub fn token_to_json(token: &Token) -> Value {
    match token {
        Token::Address(address) => json!(format!("{address:#x}")),
        Token::FixedBytes(bytes) | Token::Bytes(bytes) => {
            json!(format!("0x{}", hex::encode(bytes)))
        }
        Token::Int(value) => json!(format_signed_int(value)),
        Token::Uint(value) => json!(value.to_string()),
        Token::Bool(value) => json!(value),
        Token::String(value) => json!(value),
        Token::FixedArray(items) | Token::Array(items) | Token::Tuple(items) => {
            Value::Array(items.iter().map(token_to_json).collect())
        }
    }
}

fn format_signed_int(value: &U256) -> String {
    if !value.bit(255) {
        return value.to_string();
    }

    let magnitude = (!*value).overflowing_add(U256::from(1u8)).0;
    format!("-{magnitude}")
}
