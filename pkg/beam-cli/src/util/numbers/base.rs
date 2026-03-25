// lint-long-file-override allow-max-lines=300
use contracts::U256;
use num_bigint::{BigInt, BigUint, Sign};

use crate::error::{Error, Result};

pub fn max_int(value: Option<&str>) -> Result<String> {
    let bits = solidity_bits(value, true)?;
    let max = (BigInt::from(1u8) << (bits - 1usize)) - BigInt::from(1u8);
    Ok(max.to_string())
}

pub fn max_uint(value: Option<&str>) -> Result<String> {
    let bits = solidity_bits(value, false)?;
    let max = (BigInt::from(1u8) << bits) - BigInt::from(1u8);
    Ok(max.to_string())
}

pub fn min_int(value: Option<&str>) -> Result<String> {
    let bits = solidity_bits(value, true)?;
    let min = -(BigInt::from(1u8) << (bits - 1usize));
    Ok(min.to_string())
}

pub fn parse_u256_value(value: &str) -> Result<U256> {
    let parsed = parse_big_int(value, None)?;
    if parsed.sign() == Sign::Minus {
        return Err(Error::InvalidNumber {
            value: value.to_string(),
        });
    }

    let magnitude = parsed.magnitude().to_bytes_be();
    if magnitude.len() > 32 {
        return Err(Error::InvalidNumber {
            value: value.to_string(),
        });
    }

    let mut bytes = [0u8; 32];
    bytes[32 - magnitude.len()..].copy_from_slice(&magnitude);
    Ok(U256::from_big_endian(&bytes))
}

pub fn shift_left(
    value: &str,
    bits: &str,
    base_in: Option<&str>,
    base_out: Option<&str>,
) -> Result<String> {
    let value = parse_big_int(value, parse_optional_base(base_in)?)?;
    let bits = parse_bit_count(bits)?;
    Ok(format_big_int(
        &(value << bits),
        parse_base_or_default(base_out, 16)?,
    ))
}

pub fn shift_right(
    value: &str,
    bits: &str,
    base_in: Option<&str>,
    base_out: Option<&str>,
) -> Result<String> {
    let value = parse_big_int(value, parse_optional_base(base_in)?)?;
    let bits = parse_bit_count(bits)?;
    Ok(format_big_int(
        &(value >> bits),
        parse_base_or_default(base_out, 16)?,
    ))
}

pub fn to_base(value: &str, base_in: Option<&str>, base_out: &str) -> Result<String> {
    let value = parse_big_int(value, parse_optional_base(base_in)?)?;
    Ok(format_big_int(&value, parse_base(base_out)?))
}

pub fn to_dec(value: &str, base_in: Option<&str>) -> Result<String> {
    let value = parse_big_int(value, parse_optional_base(base_in)?)?;
    Ok(value.to_string())
}

pub fn to_hex(value: &str, base_in: Option<&str>) -> Result<String> {
    let value = parse_big_int(value, parse_optional_base(base_in)?)?;
    Ok(format_big_int(&value, 16))
}

pub fn to_int256(value: &str) -> Result<String> {
    let value = parse_big_int(value, None)?;
    let min = -(BigInt::from(1u8) << 255usize);
    let max = (BigInt::from(1u8) << 255usize) - BigInt::from(1u8);
    if value < min || value > max {
        return Err(Error::InvalidNumber {
            value: value.to_string(),
        });
    }

    let twos_complement = if value.sign() == Sign::Minus {
        (BigInt::from(1u8) << 256usize) + value
    } else {
        value
    };
    let encoded = twos_complement.magnitude().to_str_radix(16);

    Ok(format!("0x{encoded:0>64}"))
}

pub fn to_uint256(value: &str) -> Result<String> {
    let value = parse_big_int(value, None)?;
    if value.sign() == Sign::Minus || value.magnitude().to_bytes_be().len() > 32 {
        return Err(Error::InvalidNumber {
            value: value.to_string(),
        });
    }

    let encoded = value.magnitude().to_str_radix(16);
    Ok(format!("0x{encoded:0>64}"))
}

fn format_big_int(value: &BigInt, base: u32) -> String {
    let prefix = match base {
        2 => "0b",
        8 => "0o",
        10 => "",
        16 => "0x",
        _ => "",
    };

    if base == 10 {
        return value.to_string();
    }

    let digits = value.magnitude().to_str_radix(base);
    if value.sign() == Sign::Minus {
        format!("-{prefix}{digits}")
    } else {
        format!("{prefix}{digits}")
    }
}

fn parse_base(value: &str) -> Result<u32> {
    match value.trim().to_ascii_lowercase().as_str() {
        "bin" | "binary" => Ok(2),
        "oct" | "octal" => Ok(8),
        "dec" | "decimal" => Ok(10),
        "hex" | "hexadecimal" => Ok(16),
        other => other.parse::<u32>().map_err(|_| Error::InvalidBase {
            value: value.to_string(),
        }),
    }
}

fn parse_base_or_default(value: Option<&str>, default: u32) -> Result<u32> {
    match value {
        Some(value) => parse_base(value),
        None => Ok(default),
    }
}

fn parse_big_int(value: &str, base_in: Option<u32>) -> Result<BigInt> {
    let value = value.trim();
    let (negative, digits) = match value.strip_prefix('-') {
        Some(stripped) => (true, stripped),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let (base, digits) = match base_in {
        Some(base) => (base, strip_prefix_for_base(digits, base)),
        None => detect_base(digits),
    };

    let magnitude =
        BigUint::parse_bytes(digits.as_bytes(), base).ok_or_else(|| Error::InvalidNumber {
            value: value.to_string(),
        })?;
    let value = BigInt::from(magnitude);

    if negative { Ok(-value) } else { Ok(value) }
}

fn parse_bit_count(value: &str) -> Result<usize> {
    value.parse::<usize>().map_err(|_| Error::InvalidBitCount {
        value: value.to_string(),
    })
}

fn parse_optional_base(value: Option<&str>) -> Result<Option<u32>> {
    value.map(parse_base).transpose()
}

fn detect_base(value: &str) -> (u32, &str) {
    if let Some(stripped) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return (16, stripped);
    }
    if let Some(stripped) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        return (2, stripped);
    }
    if let Some(stripped) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        return (8, stripped);
    }

    (10, value)
}

fn solidity_bits(value: Option<&str>, signed: bool) -> Result<usize> {
    let default = if signed { "int256" } else { "uint256" };
    let value = value.unwrap_or(default);
    let prefix = if signed { "int" } else { "uint" };
    let Some(bits) = value.strip_prefix(prefix) else {
        return Err(Error::InvalidIntegerType {
            value: value.to_string(),
        });
    };
    let bits = if bits.is_empty() {
        256
    } else {
        bits.parse::<usize>()
            .map_err(|_| Error::InvalidIntegerType {
                value: value.to_string(),
            })?
    };

    if bits == 0 || bits > 256 || bits % 8 != 0 {
        return Err(Error::InvalidIntegerType {
            value: value.to_string(),
        });
    }

    Ok(bits)
}

fn strip_prefix_for_base(value: &str, base: u32) -> &str {
    match base {
        2 => value
            .strip_prefix("0b")
            .or_else(|| value.strip_prefix("0B"))
            .unwrap_or(value),
        8 => value
            .strip_prefix("0o")
            .or_else(|| value.strip_prefix("0O"))
            .unwrap_or(value),
        16 => value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
            .unwrap_or(value),
        _ => value,
    }
}
