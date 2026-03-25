use num_bigint::BigUint;
use rlp::{Rlp, RlpStream};
use serde_json::Value;

use crate::{
    error::{Error, Result},
    util::bytes::{decode_hex, hex_encode},
};

pub fn from_rlp(value: &str, as_int: bool) -> Result<Value> {
    let bytes = decode_hex(value)?;
    decode_value(Rlp::new(&bytes), as_int)
}

pub fn to_rlp(value: &str) -> Result<String> {
    let trimmed = value.trim();

    if trimmed.starts_with('[') {
        let json = serde_json::from_str::<Value>(trimmed).map_err(|_| Error::InvalidRlpValue {
            value: value.to_string(),
        })?;
        let mut stream = RlpStream::new();
        append_value(&mut stream, &json)?;
        return Ok(hex_encode(&stream.out()));
    }

    let bytes = decode_hex(trimmed)?;
    let mut stream = RlpStream::new();
    stream.append(&bytes);
    Ok(hex_encode(&stream.out()))
}

fn append_value(stream: &mut RlpStream, value: &Value) -> Result<()> {
    match value {
        Value::Array(items) => {
            stream.begin_list(items.len());
            for item in items {
                append_value(stream, item)?;
            }
            Ok(())
        }
        Value::String(value) => {
            let bytes = decode_hex(value)?;
            stream.append(&bytes);
            Ok(())
        }
        _ => Err(Error::InvalidRlpValue {
            value: value.to_string(),
        }),
    }
}

fn decode_value(value: Rlp<'_>, as_int: bool) -> Result<Value> {
    if value.is_list() {
        let count = value.item_count().map_err(|_| Error::InvalidRlpValue {
            value: hex_encode(value.as_raw()),
        })?;
        let mut items = Vec::with_capacity(count);

        for index in 0..count {
            items.push(decode_value(
                value.at(index).map_err(|_| Error::InvalidRlpValue {
                    value: hex_encode(value.as_raw()),
                })?,
                as_int,
            )?);
        }

        return Ok(Value::Array(items));
    }

    let data = value.data().map_err(|_| Error::InvalidRlpValue {
        value: hex_encode(value.as_raw()),
    })?;
    if as_int {
        let number = BigUint::from_bytes_be(data);
        Ok(Value::String(number.to_str_radix(10)))
    } else {
        Ok(Value::String(hex_encode(data)))
    }
}
