use ethnum::U256;
use serde::{Deserialize, Deserializer, Serializer};

pub(super) fn serialize<S>(u: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    hex::serde::serialize(u.to_be_bytes(), serializer)
}

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let vec = hex::decode(s.trim_start_matches("0x")).map_err(serde::de::Error::custom)?;
    let bytes =
        <[u8; 32]>::try_from(vec).map_err(|_| serde::de::Error::custom("Invalid length"))?;
    Ok(U256::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use test_strategy::proptest;

    use crate::Element;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Example {
        element: Element,
    }

    #[proptest]
    fn canonical_element_serialize_bijection(mut element: Element) {
        element.canonicalize();

        let value = serde_json::to_value(element).unwrap();
        let element_again: Element = serde_json::from_value(value).unwrap();

        assert_eq!(element, element_again);
    }

    #[proptest]
    fn elements_produce_identical_base_before_after_serialize(element: Element) {
        let base = element.to_base();

        let value = serde_json::to_value(element).unwrap();
        let element_again: Element = serde_json::from_value(value).unwrap();

        let base_again = element_again.to_base();

        assert_eq!(base, base_again);
    }
}
