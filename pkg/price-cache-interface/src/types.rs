use std::fmt;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use currency::Currency;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TokenIdentifier {
    Symbol { symbol: String },
    Address { network: String, address: String },
}

impl TokenIdentifier {
    #[must_use]
    pub fn symbol_key(&self) -> Option<&str> {
        match self {
            Self::Symbol { symbol } => Some(symbol.as_str()),
            Self::Address { .. } => None,
        }
    }

    #[must_use]
    pub fn network_value(&self) -> &str {
        match self {
            Self::Symbol { .. } => "global",
            Self::Address { network, .. } => network.as_str(),
        }
    }

    #[must_use]
    pub fn contract_address(&self) -> Option<&str> {
        match self {
            Self::Symbol { .. } => None,
            Self::Address { address, .. } => Some(address.as_str()),
        }
    }
}

impl fmt::Display for TokenIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Symbol { symbol } => write!(f, "{symbol}"),
            Self::Address { network, address } => write!(f, "{network}:{address}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPrice {
    #[serde(with = "bigdecimal_as_string")]
    pub value: BigDecimal,
    pub currency: Currency,
    pub last_updated_at: DateTime<Utc>,
}

impl Default for TokenPrice {
    fn default() -> Self {
        Self {
            value: BigDecimal::from(0),
            currency: Currency::USD,
            last_updated_at: Utc::now(),
        }
    }
}

mod bigdecimal_as_string {
    use bigdecimal::BigDecimal;
    use serde::{Deserialize, Deserializer, Serializer, de::Error as DeError};
    use std::str::FromStr;

    pub fn serialize<S>(value: &BigDecimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        BigDecimal::from_str(&s).map_err(|err| DeError::custom(err.to_string()))
    }
}
