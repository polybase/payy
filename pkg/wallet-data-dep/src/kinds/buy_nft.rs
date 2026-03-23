use element::Element;
use serde::{Deserialize, Serialize};

use crate::StoredNote;

// BuyNft Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "kebab-case")]
pub enum WalletActivityBuyNftStage {
    Init(BuyNftInitData),
    RequestPayment(Box<BuyNftRequestPaymentCombinedData>),
}

impl WalletActivityBuyNftStage {
    pub fn value(&self) -> Element {
        match self {
            WalletActivityBuyNftStage::Init(d) => d.value,
            WalletActivityBuyNftStage::RequestPayment(d) => d.init_data.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuyNftInitData {
    pub payment_id: Option<String>,
    pub value: Element,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuyNftRequestPaymentCombinedData {
    #[serde(flatten)]
    pub init_data: BuyNftInitData,
    #[serde(flatten)]
    pub request_payment_data: BuyNftRequestPaymentData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuyNftRequestPaymentData {
    pub url: String,
    pub private_key: String,
    pub note: StoredNote,
}
