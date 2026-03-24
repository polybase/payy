use element::Element;
use ethereum_types::Address;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{PayyData, SnarkWitness, StoredNote, activity::WalletActivityBase};

// Burn Activity Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletActivityBurn {
    #[serde(flatten)]
    pub base: WalletActivityBase,
    #[serde(flatten)]
    pub stage: WalletActivityBurnStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivityBurnStage {
    Init(BurnInitData),
    Merge(BurnMergeCombinedData),
    Proof(BurnProofCombinedData),
    Ethereum(BurnEthereumCombinedData),
    Rollup(BurnRollupCombinedData),
    Success(BurnSuccessCombinedData),
}

impl WalletActivityBurnStage {
    pub fn value(&self) -> Element {
        match &self {
            WalletActivityBurnStage::Init(init) => init.value,
            WalletActivityBurnStage::Merge(merge) => merge.init_data.value,
            WalletActivityBurnStage::Proof(proof) => proof.init_data.value,
            WalletActivityBurnStage::Ethereum(ethereum) => ethereum.init_data.value,
            WalletActivityBurnStage::Rollup(rollup) => rollup.init_data.value,
            WalletActivityBurnStage::Success(success) => success.init_data.value,
        }
    }

    pub fn eth_address(&self) -> Address {
        let str = match &self {
            WalletActivityBurnStage::Init(init) => &init.ethaddress,
            WalletActivityBurnStage::Merge(merge) => &merge.init_data.ethaddress,
            WalletActivityBurnStage::Proof(proof) => &proof.init_data.ethaddress,
            WalletActivityBurnStage::Ethereum(ethereum) => &ethereum.init_data.ethaddress,
            WalletActivityBurnStage::Rollup(rollup) => &rollup.init_data.ethaddress,
            WalletActivityBurnStage::Success(success) => &success.init_data.ethaddress,
        };
        *str
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BurnRouterType {
    Router,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnRouterData {
    #[serde(rename = "type")]
    pub router_type: BurnRouterType,
    pub router: String,
    pub router_calldata: String,
    pub return_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnInitData {
    #[serde(deserialize_with = "deserialize_address")]
    pub ethaddress: Address,
    pub value: Element,
    pub kind: Option<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<BurnRouterData>,
}

pub fn deserialize_address<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Trim off the 0x
    // Trim off the 0x prefix if present
    let s = if s.starts_with("0x") {
        s.trim_start_matches("0x").to_string()
    } else {
        s
    };

    // Check if the string is a valid hex representation
    let s = s.trim_start_matches("0x");

    // Remove any padding (leading zeros beyond 40 characters)
    let s = if s.len() > 40 { &s[s.len() - 40..] } else { s };

    // Parse as Address
    Address::from_str(&format!("0x{s}")).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnMergeCombinedData {
    #[serde(flatten)]
    pub init_data: BurnInitData,
    #[serde(flatten)]
    pub merge_data: BurnMergeData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnMergeData {
    pub note: StoredNote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnProofCombinedData {
    #[serde(flatten)]
    pub init_data: BurnInitData,
    #[serde(flatten)]
    pub merge_data: BurnMergeData,
    #[serde(flatten)]
    pub proof_data: BurnProofData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnProofData {
    pub snark: SnarkWitness,
    pub proof: String,
    pub root: Option<Element>,
    pub signature: Element,
    pub nullifier: Element,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnEthereumCombinedData {
    #[serde(flatten)]
    pub init_data: BurnInitData,
    #[serde(flatten)]
    pub merge_data: BurnMergeData,
    #[serde(flatten)]
    pub proof_data: BurnProofData,
    #[serde(flatten)]
    pub ethereum_data: BurnEthereumData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnEthereumData {
    pub txn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnRollupCombinedData {
    #[serde(flatten)]
    pub init_data: BurnInitData,
    #[serde(flatten)]
    pub merge_data: BurnMergeData,
    #[serde(flatten)]
    pub proof_data: BurnProofData,
    #[serde(flatten)]
    pub ethereum_data: BurnEthereumData,
    #[serde(flatten)]
    pub rollup_data: BurnRollupData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnRollupData {
    pub height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payy: Option<PayyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnSuccessCombinedData {
    #[serde(flatten)]
    pub init_data: BurnInitData,
    #[serde(flatten)]
    pub merge_data: BurnMergeData,
    #[serde(flatten)]
    pub ethereum_data: BurnEthereumData,
    #[serde(flatten)]
    pub rollup_data: BurnRollupData,
    pub snark: Option<()>,
    pub proof: Option<()>,
}
