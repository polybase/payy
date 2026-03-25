use contracts::U256;

use crate::{
    error::{Error, Result},
    evm::parse_units,
};

use super::base::parse_u256_value;

pub fn format_units_value(value: &str, unit: &str) -> Result<String> {
    let decimals = unit_decimals(unit)?;
    let value = parse_u256_value(value)?;
    Ok(format_units_cast(value, decimals))
}

pub fn from_fixed_point(decimals: &str, value: &str) -> Result<String> {
    let decimals = unit_decimals(decimals)?;
    let value = parse_units(value, decimals)?;
    Ok(value.to_string())
}

pub fn from_wei(value: &str, unit: Option<&str>) -> Result<String> {
    let decimals = unit_decimals(unit.unwrap_or("eth"))?;
    let value = parse_u256_value(value)?;
    Ok(format_units_fixed(value, decimals, true))
}

pub fn parse_units_value(value: &str, unit: &str) -> Result<String> {
    let decimals = unit_decimals(unit)?;
    let value = parse_units(value, decimals)?;
    Ok(value.to_string())
}

pub fn to_fixed_point(decimals: &str, value: &str) -> Result<String> {
    let decimals = unit_decimals(decimals)?;
    let value = parse_u256_value(value)?;
    Ok(format_units_fixed(value, decimals, false))
}

pub fn to_unit(value: &str, unit: Option<&str>) -> Result<String> {
    let target_unit = unit.unwrap_or("wei");
    let target_decimals = unit_decimals(target_unit)?;
    let (amount, source_unit) = parse_amount_with_unit(value, "wei")?;
    let source_decimals = unit_decimals(&source_unit)?;
    let scaled = parse_units(&amount, source_decimals)?;
    Ok(format_units_fixed(scaled, target_decimals, false))
}

pub fn to_wei(value: &str, unit: Option<&str>) -> Result<String> {
    let (amount, source_unit) = parse_amount_with_unit(value, unit.unwrap_or("eth"))?;
    let source_decimals = unit_decimals(&source_unit)?;
    let scaled = parse_units(&amount, source_decimals)?;
    Ok(scaled.to_string())
}

fn format_units_cast(value: U256, decimals: usize) -> String {
    if decimals == 0 {
        return value.to_string();
    }

    let raw = value.to_string();
    let split = raw.len().saturating_sub(decimals);
    let whole = if split == 0 { "0" } else { &raw[..split] };
    let fraction = if split == 0 {
        format!("{}{}", "0".repeat(decimals - raw.len()), raw)
    } else {
        raw[split..].to_string()
    };

    if fraction.chars().all(|ch| ch == '0') {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    }
}

fn format_units_fixed(value: U256, decimals: usize, force_fraction: bool) -> String {
    if decimals == 0 {
        let whole = value.to_string();
        return if force_fraction {
            format!("{whole}.0")
        } else {
            whole
        };
    }

    let raw = value.to_string();
    let split = raw.len().saturating_sub(decimals);
    let whole = if split == 0 { "0" } else { &raw[..split] };
    let fraction = if split == 0 {
        format!("{}{}", "0".repeat(decimals - raw.len()), raw)
    } else {
        raw[split..].to_string()
    };

    format!("{whole}.{fraction}")
}

fn parse_amount_with_unit(value: &str, default_unit: &str) -> Result<(String, String)> {
    let value = value.trim();
    if value.is_empty() {
        return Err(Error::InvalidAmount {
            value: value.to_string(),
        });
    }

    let split = value
        .char_indices()
        .find(|(_, ch)| ch.is_ascii_alphabetic())
        .map(|(index, _)| index);
    let Some(split) = split else {
        return Ok((value.to_string(), default_unit.to_string()));
    };

    let amount = value[..split].trim();
    let unit = value[split..].trim();
    if amount.is_empty() || unit.is_empty() {
        return Err(Error::InvalidUnit {
            value: value.to_string(),
        });
    }

    Ok((amount.to_string(), unit.to_string()))
}

fn unit_decimals(value: &str) -> Result<usize> {
    match value.trim().to_ascii_lowercase().as_str() {
        "wei" => Ok(0),
        "gwei" => Ok(9),
        "eth" | "ether" => Ok(18),
        other => other.parse::<usize>().map_err(|_| Error::InvalidUnit {
            value: value.to_string(),
        }),
    }
}
