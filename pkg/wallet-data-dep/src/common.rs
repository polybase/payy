use crate::util::deserialize_optional_element;
use element::Element;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PayyData {
    pub height: u64,
    #[serde(default, deserialize_with = "deserialize_optional_element")]
    pub root: Option<Element>,
    #[serde(default, deserialize_with = "deserialize_optional_element")]
    pub txn: Option<Element>,
}

pub struct PayyDataDefault {
    pub height: u64,
    pub root: Element,
    pub txn: Element,
}

impl From<&PayyData> for PayyDataDefault {
    fn from(data: &PayyData) -> Self {
        Self {
            height: data.height,
            root: data.root.unwrap_or_default(),
            txn: data.txn.unwrap_or_default(),
        }
    }
}

impl From<&Option<PayyData>> for PayyDataDefault {
    fn from(data: &Option<PayyData>) -> Self {
        match data {
            Some(data) => Self {
                height: data.height,
                root: data.root.unwrap_or_default(),
                txn: data.txn.unwrap_or_default(),
            },
            None => Self {
                height: 0,
                root: Element::default(),
                txn: Element::default(),
            },
        }
    }
}
