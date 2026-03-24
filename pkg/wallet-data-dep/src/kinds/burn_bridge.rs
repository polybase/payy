use element::Element;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "kebab-case")]
pub enum WalletActivityBurnBridgeStage {
    Init(BurnBridgeInitData),
    BurnToAddress(BurnBridgeBurnToAddressCombinedData),
    Success(BurnBridgeSuccessCombinedData),
    Failed(BurnBridgeFailedCombinedData),
}

impl WalletActivityBurnBridgeStage {
    pub fn value(&self) -> Element {
        match self {
            WalletActivityBurnBridgeStage::Init(data) => data.value,
            WalletActivityBurnBridgeStage::BurnToAddress(data) => data.init_data.value,
            WalletActivityBurnBridgeStage::Success(data) => data.init_data.value,
            WalletActivityBurnBridgeStage::Failed(data) => data.init_data.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BurnBridgeProvider {
    Across,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BurnBridgeInitData {
    pub provider: BurnBridgeProvider,
    pub chain_id: u64,
    pub to: String,
    pub output_token: String,
    pub value: Element,
    pub minimum_output_value: Element,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BurnBridgeBurnToAddressData {
    pub child_burn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnBridgeBurnToAddressCombinedData {
    #[serde(flatten)]
    pub init_data: BurnBridgeInitData,
    #[serde(flatten)]
    pub burn_to_address_data: BurnBridgeBurnToAddressData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnBridgeSuccessData {
    pub txn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnBridgeSuccessCombinedData {
    #[serde(flatten)]
    pub init_data: BurnBridgeInitData,
    #[serde(flatten)]
    pub success_data: BurnBridgeSuccessData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnBridgeFailedData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnBridgeFailedCombinedData {
    #[serde(flatten)]
    pub init_data: BurnBridgeInitData,
    #[serde(flatten)]
    pub burn_to_address_data: BurnBridgeBurnToAddressData,
    #[serde(flatten)]
    pub failed_data: BurnBridgeFailedData,
}
