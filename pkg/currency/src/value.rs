use element::Element;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer, de::Error as SerdeDeError,
    ser::SerializeStruct,
};

use crate::{currency::Currency, error::Result};

// Represents a value denominated in a specific currency and amount pairing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrencyValue {
    currency: Currency,
    amount: Element,
}

impl CurrencyValue {
    #[must_use]
    pub const fn new(currency: Currency, amount: Element) -> Self {
        Self { currency, amount }
    }

    #[must_use]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    #[must_use]
    pub fn amount(&self) -> Element {
        self.amount
    }

    #[must_use]
    pub fn into_parts(self) -> (Currency, Element) {
        (self.currency, self.amount)
    }

    pub fn try_from_decimal_string(currency: Currency, value: &str) -> Result<Self> {
        let amount = currency.try_from_decimal_string(value)?;
        Ok(Self::new(currency, amount))
    }

    #[must_use]
    pub fn to_decimal_string(&self) -> String {
        self.currency.to_decimal_string(&self.amount)
    }
}

impl Serialize for CurrencyValue {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CurrencyValue", 2)?;
        state.serialize_field("currency", &self.currency)?;
        state.serialize_field("value", &self.currency.to_decimal_string(&self.amount))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for CurrencyValue {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            currency: Currency,
            value: String,
        }

        let Helper { currency, value } = Helper::deserialize(deserializer)?;
        let amount = currency
            .try_from_decimal_string(&value)
            .map_err(|err| SerdeDeError::custom(err.to_string()))?;
        Ok(CurrencyValue::new(currency, amount))
    }
}
