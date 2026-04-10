use element::Element;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WalletActivityTxnStage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivitySwapData {
    pub from_amount: Element,
    pub from_note_kind: Element,
    pub to_note_kind: Element,
    pub txn_id: Option<Uuid>,
    pub private_key: Element,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivitySwapFundData {
    #[serde(flatten)]
    pub swap: WalletActivitySwapData,
    pub txn: Box<WalletActivityTxnStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivitySwapSuccessData {
    #[serde(flatten)]
    pub swap: WalletActivitySwapData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivitySwapFailData {
    #[serde(flatten)]
    pub swap: WalletActivitySwapData,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "camelCase")]
pub enum WalletActivitySwapStage {
    CreateSwapTxn(WalletActivitySwapData),
    FundSwap(WalletActivitySwapFundData),
    SubmitNote(WalletActivitySwapData),
    WaitForCredit(WalletActivitySwapData),
    Success(WalletActivitySwapSuccessData),
    Fail(WalletActivitySwapFailData),
}
