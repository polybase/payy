use element::Element;
use serde::{Deserialize, Deserializer};

// Custom deserializer function that converts failures to None
pub fn deserialize_optional_element<'de, D>(deserializer: D) -> Result<Option<Element>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let result: Result<Element, _> = Deserialize::deserialize(deserializer);
    Ok(result.ok())
}

pub fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

pub fn deserialize_option_or_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match T::deserialize(value) {
        Ok(t) => Ok(Some(t)),
        Err(_) => Ok(None),
    }
}
