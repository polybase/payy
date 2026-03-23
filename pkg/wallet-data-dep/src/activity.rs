use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::util::deserialize_null_default;
use crate::{
    WalletActivityBurnBridgeStage, WalletActivityBurnStage, WalletActivityBuyNftStage,
    WalletActivityClaim, WalletActivityMergeStage, WalletActivityMintStage,
    WalletActivityRampStage, WalletActivityReceiveStage, WalletActivitySendNoteStage,
    WalletActivitySendStage, WalletActivitySupportStage, WalletActivitySwapStage,
    WalletActivityTxnStage,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityBase {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub result: WalletActivityResultStatus,
    #[serde(with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    pub user_cancel: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub error_cycles: u32,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub ok_cycles: u32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WalletActivityResultStatus {
    #[default]
    Pending,
    Success,
    Error,
    Declined,
    Cancelled,
    Onhold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletActivity {
    #[serde(flatten)]
    pub base: WalletActivityBase,
    #[serde(flatten)]
    pub kind: WalletActivityKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WalletActivityKind {
    Merge(WalletActivityMergeStage),
    BuyNft(WalletActivityBuyNftStage),
    Mint(WalletActivityMintStage),
    Receive(WalletActivityReceiveStage),
    Send(WalletActivitySendStage),
    Claim(WalletActivityClaim),
    SendNote(WalletActivitySendNoteStage),
    Txn(WalletActivityTxnStage),
    Swap(WalletActivitySwapStage),
    Burn(WalletActivityBurnStage),
    BurnBridge(WalletActivityBurnBridgeStage),
    Ramp(Box<WalletActivityRampStage>),
    Support(WalletActivitySupportStage),
}

impl WalletActivityKind {
    pub fn id(&self) -> Option<Uuid> {
        match self {
            WalletActivityKind::Ramp(ramp) => ramp.id(),
            _ => None,
        }
    }
}
