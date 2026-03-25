// lint-long-file-override allow-max-lines=300
use contracts::Address;

use crate::{
    error::{Error, Result},
    util::hash::checksum_address,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrettyCalldata {
    pub remainder: Option<String>,
    pub selector: Option<String>,
    pub words: Vec<String>,
}

pub fn concat_hex(values: &[String]) -> Result<String> {
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend_from_slice(&decode_hex(value)?);
    }
    Ok(hex_encode(&bytes))
}

pub fn decode_hex(value: &str) -> Result<Vec<u8>> {
    let normalized = normalize_hexdata(value)?;
    let raw = normalized.strip_prefix("0x").unwrap_or(&normalized);

    hex::decode(raw).map_err(|_| Error::InvalidHexData {
        value: value.trim().to_string(),
    })
}

pub fn format_bytes32_string(value: &str) -> Result<String> {
    if value.len() > 32 {
        return Err(Error::InvalidBytes32Value {
            value: value.to_string(),
        });
    }

    let mut bytes = value.as_bytes().to_vec();
    bytes.resize(32, 0u8);
    Ok(hex_encode(&bytes))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

pub fn hex_to_ascii(value: &str) -> Result<String> {
    let bytes = decode_hex(value)?;
    if !bytes.iter().all(u8::is_ascii) {
        return Err(Error::InvalidAsciiData {
            value: value.to_string(),
        });
    }

    Ok(bytes.into_iter().map(char::from).collect())
}

pub fn hex_to_utf8(value: &str) -> Result<String> {
    let bytes = decode_hex(value)?;
    String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8Data)
}

pub fn normalize_hexdata(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let mut normalized = String::new();

    for part in trimmed.split(':') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let part = part.strip_prefix("0x").unwrap_or(part);
        if !part.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(Error::InvalidHexData {
                value: trimmed.to_string(),
            });
        }

        normalized.push_str(&part.to_ascii_lowercase());
    }

    if normalized.len() % 2 == 1 {
        normalized.insert(0, '0');
    }

    Ok(format!("0x{normalized}"))
}

pub fn parse_bytes32_address(value: &str) -> Result<String> {
    let bytes = decode_hex(value)?;
    if bytes.len() != 32 || bytes[..12].iter().any(|byte| *byte != 0) {
        return Err(Error::InvalidBytes32Value {
            value: value.to_string(),
        });
    }

    Ok(checksum_address(Address::from_slice(&bytes[12..]), None))
}

pub fn parse_bytes32_string(value: &str) -> Result<String> {
    let bytes = decode_hex(value)?;
    if bytes.len() != 32 {
        return Err(Error::InvalidBytes32Value {
            value: value.to_string(),
        });
    }

    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if bytes[end..].iter().any(|byte| *byte != 0) {
        return Err(Error::InvalidBytes32Value {
            value: value.to_string(),
        });
    }

    String::from_utf8(bytes[..end].to_vec()).map_err(|_| Error::InvalidUtf8Data)
}

pub fn pad_hex(value: &str, len: usize, right: bool) -> Result<String> {
    let bytes = decode_hex(value)?;
    if bytes.len() > len {
        return Err(Error::InvalidHexData {
            value: value.to_string(),
        });
    }

    let mut padded = vec![0u8; len];
    if right {
        padded[..bytes.len()].copy_from_slice(&bytes);
    } else {
        let start = len - bytes.len();
        padded[start..].copy_from_slice(&bytes);
    }

    Ok(hex_encode(&padded))
}

pub fn pretty_calldata(value: &str) -> Result<PrettyCalldata> {
    let bytes = decode_hex(value)?;
    let (selector, payload) = if bytes.len() >= 4 {
        (Some(hex_encode(&bytes[..4])), &bytes[4..])
    } else {
        (None, bytes.as_slice())
    };
    let mut words = Vec::new();
    let mut offset = 0usize;

    while offset + 32 <= payload.len() {
        words.push(hex_encode(&payload[offset..offset + 32]));
        offset += 32;
    }

    Ok(PrettyCalldata {
        remainder: (offset < payload.len()).then(|| hex_encode(&payload[offset..])),
        selector,
        words,
    })
}

pub fn to_bytes32(value: &str) -> Result<String> {
    pad_hex(value, 32, true)
}

pub fn utf8_to_hex(value: &str) -> String {
    hex_encode(value.as_bytes())
}
