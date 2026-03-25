use contracts::U256;

use crate::error::{Error, Result};

const MAX_UNIT_DECIMALS: usize = 77;

pub fn validate_unit_decimals(decimals: usize) -> Result<()> {
    let _ = ten_pow(decimals)?;
    Ok(())
}

pub fn parse_units(value: &str, decimals: usize) -> Result<U256> {
    let value = value.trim();
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() && fraction.is_empty() {
        return Err(Error::InvalidAmount {
            value: value.to_string(),
        });
    }
    if fraction.len() > decimals || !valid_decimal(whole) || !valid_decimal(fraction) {
        return Err(Error::InvalidAmount {
            value: value.to_string(),
        });
    }

    let base = ten_pow(decimals)?;
    let whole = parse_u256(whole)?;
    let mut fraction_padded = fraction.to_string();
    fraction_padded.push_str(&"0".repeat(decimals - fraction.len()));
    let fraction = parse_u256(&fraction_padded)?;

    whole
        .checked_mul(base)
        .and_then(|scaled| scaled.checked_add(fraction))
        .ok_or_else(|| Error::InvalidAmount {
            value: value.to_string(),
        })
}

pub fn format_units(value: U256, decimals: u8) -> String {
    let raw = value.to_string();
    if decimals == 0 {
        return raw;
    }
    if raw.len() <= decimals as usize {
        return trim_fraction(format!(
            "0.{}{}",
            "0".repeat(decimals as usize - raw.len()),
            raw
        ));
    }

    let split = raw.len() - decimals as usize;
    trim_fraction(format!("{}.{}", &raw[..split], &raw[split..]))
}

fn parse_u256(value: &str) -> Result<U256> {
    if value.is_empty() {
        return Ok(U256::zero());
    }
    U256::from_dec_str(value).map_err(|_| Error::InvalidAmount {
        value: value.to_string(),
    })
}

fn ten_pow(decimals: usize) -> Result<U256> {
    U256::from(10u8)
        .checked_pow(U256::from(decimals))
        .ok_or_else(|| Error::UnsupportedDecimals {
            decimals,
            max: MAX_UNIT_DECIMALS,
        })
}

fn trim_fraction(value: String) -> String {
    if !value.contains('.') {
        return value;
    }

    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn valid_decimal(value: &str) -> bool {
    value.chars().all(|ch| ch.is_ascii_digit())
}
