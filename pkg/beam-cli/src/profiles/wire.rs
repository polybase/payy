use serde::{Deserialize, Serialize};
use web3::types::{Bytes, H256, SignedTransaction, TransactionParameters, U64};

use crate::profiles::{Error, Result, model::PublicSigningIntent};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DaemonRequest {
    Ping {
        token: String,
    },
    Lock {
        token: String,
    },
    Rollback {
        token: String,
        ledger_id: String,
    },
    Sign {
        token: String,
        intent: Box<PublicSigningIntent>,
        transaction: Box<WireTransaction>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DaemonResponse {
    Ok,
    Signed {
        ledger_id: String,
        transaction: WireSignedTransaction,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WireTransaction {
    pub nonce: Option<String>,
    pub to: Option<String>,
    pub gas: String,
    pub gas_price: Option<String>,
    pub value: String,
    pub data: String,
    pub chain_id: Option<u64>,
    pub transaction_type: Option<u64>,
    pub max_fee_per_gas: Option<String>,
    pub max_priority_fee_per_gas: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WireSignedTransaction {
    pub message_hash: String,
    pub v: u64,
    pub r: String,
    pub s: String,
    pub raw_transaction: String,
    pub transaction_hash: String,
}

impl WireTransaction {
    pub fn from_parameters(transaction: &TransactionParameters) -> Self {
        Self {
            nonce: transaction.nonce.map(|value| value.to_string()),
            to: transaction.to.map(|value| format!("{value:#x}")),
            gas: transaction.gas.to_string(),
            gas_price: transaction.gas_price.map(|value| value.to_string()),
            value: transaction.value.to_string(),
            data: format!("0x{}", hex::encode(&transaction.data.0)),
            chain_id: transaction.chain_id,
            transaction_type: transaction.transaction_type.map(|value| value.as_u64()),
            max_fee_per_gas: transaction.max_fee_per_gas.map(|value| value.to_string()),
            max_priority_fee_per_gas: transaction
                .max_priority_fee_per_gas
                .map(|value| value.to_string()),
        }
    }

    pub fn to_parameters(&self) -> Result<TransactionParameters> {
        Ok(TransactionParameters {
            nonce: parse_optional_u256(self.nonce.as_deref())?,
            to: match self.to.as_deref() {
                Some(value) => Some(parse_address(value)?),
                None => None,
            },
            gas: parse_u256(&self.gas)?,
            gas_price: parse_optional_u256(self.gas_price.as_deref())?,
            value: parse_u256(&self.value)?,
            data: Bytes(parse_hex_data(&self.data)?),
            chain_id: self.chain_id,
            transaction_type: self.transaction_type.map(U64::from),
            max_fee_per_gas: parse_optional_u256(self.max_fee_per_gas.as_deref())?,
            max_priority_fee_per_gas: parse_optional_u256(
                self.max_priority_fee_per_gas.as_deref(),
            )?,
            ..Default::default()
        })
    }
}

impl WireSignedTransaction {
    pub fn from_signed(transaction: &SignedTransaction) -> Self {
        Self {
            message_hash: format!("{:#x}", transaction.message_hash),
            v: transaction.v,
            r: format!("{:#x}", transaction.r),
            s: format!("{:#x}", transaction.s),
            raw_transaction: format!("0x{}", hex::encode(&transaction.raw_transaction.0)),
            transaction_hash: format!("{:#x}", transaction.transaction_hash),
        }
    }

    pub fn to_signed(&self) -> Result<SignedTransaction> {
        Ok(SignedTransaction {
            message_hash: parse_h256(&self.message_hash)?,
            v: self.v,
            r: parse_h256(&self.r)?,
            s: parse_h256(&self.s)?,
            raw_transaction: Bytes(parse_hex_data(&self.raw_transaction)?),
            transaction_hash: parse_h256(&self.transaction_hash)?,
        })
    }
}

fn parse_optional_u256(value: Option<&str>) -> Result<Option<contracts::U256>> {
    value.map(parse_u256).transpose()
}

fn parse_u256(value: &str) -> Result<contracts::U256> {
    if let Some(value) = value.strip_prefix("0x") {
        return contracts::U256::from_str_radix(value, 16).map_err(|_| Error::InvalidAmount {
            value: format!("0x{value}"),
        });
    }
    contracts::U256::from_dec_str(value).map_err(|_| Error::InvalidAmount {
        value: value.to_string(),
    })
}

fn parse_address(value: &str) -> Result<contracts::Address> {
    value.parse().map_err(|_| Error::PolicyDenied {
        reason: format!("invalid transaction address {value}"),
    })
}

fn parse_h256(value: &str) -> Result<H256> {
    value.parse().map_err(|_| Error::PolicyDenied {
        reason: format!("invalid signed transaction hash {value}"),
    })
}

fn parse_hex_data(value: &str) -> Result<Vec<u8>> {
    hex::decode(value.strip_prefix("0x").unwrap_or(value)).map_err(|_| Error::PolicyDenied {
        reason: "invalid transaction data".to_string(),
    })
}
