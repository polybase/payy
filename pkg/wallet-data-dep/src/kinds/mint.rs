use element::Element;
use serde::{Deserialize, Serialize};

use crate::{
    PayyData, PayyDataDefault, SnarkWitness, StoredNote, util::deserialize_option_or_none,
};

// Mint Activity Types

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivityMintStage {
    Deposit(MintInitData),
    Init(MintInitData),
    Proof(MintProofCombinedData),
    Ethereum(MintEthereumCombinedData),
    Rollup(MintRollupCombinedData),
    Success(MintSuccessCombinedData),
}

impl WalletActivityMintStage {
    pub fn value(&self) -> Element {
        match self {
            WalletActivityMintStage::Init(init) => init.value,
            WalletActivityMintStage::Deposit(deposit) => deposit.value,
            WalletActivityMintStage::Proof(proof) => proof.init_data.value,
            WalletActivityMintStage::Ethereum(ethereum) => ethereum.init_data.value,
            WalletActivityMintStage::Rollup(rollup) => rollup.init_data.value,
            WalletActivityMintStage::Success(success) => success.init_data.value,
        }
    }

    pub fn txn_receipt(&self) -> Option<PayyDataDefault> {
        match &self {
            WalletActivityMintStage::Rollup(rollup) => {
                Some(PayyDataDefault::from(&rollup.rollup_data.payy))
            }
            WalletActivityMintStage::Success(success) => {
                Some(PayyDataDefault::from(&success.rollup_data.payy))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MintInitData {
    pub to: Element,
    pub value: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_option_or_none", default)]
    pub provider: Option<WalletActivityMintProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WalletActivityMintProviderType {
    Mayan,
    Across,
    #[serde(rename = "polygon-usdc")]
    PolygonUsdc,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityMintProvider {
    pub provider: WalletActivityMintProviderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txn_success: Option<bool>,
    pub estimated_value: Element,
    pub from_chain: String,
    pub from_token: String,
    pub from_value: u64,
    pub eta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintProofCombinedData {
    #[serde(flatten)]
    pub init_data: MintInitData,
    #[serde(flatten)]
    pub proof_data: MintProofData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintProofData {
    pub note: StoredNote,
    pub snark: SnarkWitness,
    pub proof: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintEthereumCombinedData {
    #[serde(flatten)]
    pub init_data: MintInitData,
    #[serde(flatten)]
    pub proof_data: MintProofData,
    #[serde(flatten)]
    pub ethereum_data: MintEthereumData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintEthereumData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintRollupCombinedData {
    #[serde(flatten)]
    pub init_data: MintInitData,
    #[serde(flatten)]
    pub ethereum_data: MintEthereumData,
    #[serde(flatten)]
    pub rollup_data: MintRollupData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintRollupData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payy: Option<PayyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintSuccessCombinedData {
    #[serde(flatten)]
    pub rollup_data: MintRollupData,
    #[serde(flatten)]
    pub init_data: MintInitData,
    #[serde(flatten)]
    pub ethereum_data: MintEthereumData,
    pub note: StoredNote,
    pub snark: Option<()>,
    pub proof: Option<()>,
}
