use contextful::ResultContextExt;
use serde_json::Value;

use crate::{
    apps::{Error as AppError, model::ActionStep},
    error::{Error, Result},
};

pub fn transaction(step: &ActionStep) -> Option<TransactionValue<'_>> {
    step.metadata
        .get("transaction")
        .and_then(Value::as_object)
        .map(TransactionValue)
}

pub struct TransactionValue<'a>(&'a serde_json::Map<String, Value>);

impl TransactionValue<'_> {
    pub fn data(&self) -> Result<&str> {
        self.string("data")
    }

    pub fn gas_limit(&self) -> Option<&str> {
        self.optional_string("gas_limit")
    }

    pub fn to(&self) -> Result<&str> {
        self.string("to")
    }

    pub fn value(&self) -> Option<&str> {
        self.optional_string("value")
    }

    fn string(&self, key: &str) -> Result<&str> {
        self.optional_string(key).ok_or_else(|| {
            Error::App(AppError::InvalidHostRequest {
                reason: format!("transaction missing {key}"),
            })
        })
    }

    fn optional_string(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }
}

pub fn parse_hex_data(value: &str) -> Result<Vec<u8>> {
    hex::decode(value.strip_prefix("0x").unwrap_or(value)).map_err(|_| Error::InvalidHexData {
        value: value.to_string(),
    })
}

pub fn parse_u256(value: &str) -> Result<contracts::U256> {
    if let Some(value) = value.strip_prefix("0x") {
        return Ok(contracts::U256::from_str_radix(value, 16).context("parse hex u256")?);
    }
    Ok(contracts::U256::from_dec_str(value).context("parse decimal u256")?)
}
