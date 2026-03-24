use element::Element;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    StoredNote, WalletActivity, WalletActivityKind, WalletActivityMintStage,
    WalletActivityResultStatus, WalletActivityTxnStage,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletState {
    pub version: String,
    pub last_update: Option<String>,
    pub address: Option<Element>,
    #[serde(default)]
    pub invalid_notes: IndexMap<Element, StoredNote>,
    #[serde(default)]
    pub unspent_notes: IndexMap<Element, StoredNote>,
    #[serde(default)]
    pub spent_notes: IndexMap<Element, StoredNote>,
    #[serde(default)]
    pub pending_notes: IndexMap<Element, StoredNote>,
    pub activity: IndexMap<String, WalletActivity>,
    #[serde(default)]
    pub registry_check_block: u64,
}

impl WalletState {
    /// Gets all possible notes in the wallet
    pub fn get_notes(&self) -> Vec<StoredNote> {
        let mut notes = Vec::new();

        // Collect from pending_notes
        notes.extend(self.pending_notes.values().cloned());

        // Collect from unspent_notes
        notes.extend(self.unspent_notes.values().cloned());

        // Collect from invalid_notes
        notes.extend(self.invalid_notes.values().cloned());

        // Look for txn notes
        for activity in self.activity.values() {
            if activity.base.result == WalletActivityResultStatus::Pending
                && let WalletActivityKind::Txn(WalletActivityTxnStage::Init(init)) = &activity.kind
            {
                // Add input notes
                notes.extend(init.inputs.clone());
                // Add output notes
                notes.extend(init.outputs.clone());
            }
            if let WalletActivityKind::Send(send) = &activity.kind
                && let Some(note) = send.note()
            {
                notes.push(note);
            }
        }

        notes
    }

    /// Gets failed Mayan and Across mint activities that may have USDC balances
    /// at their derived deposit addresses. Returns activities with their original IDs.
    pub fn get_failed_mints(&self) -> Vec<(String, &WalletActivity)> {
        let mut failed_mints = Vec::new();

        for (activity_id, activity) in &self.activity {
            // Only look at failed activities (not success)
            if activity.base.result != WalletActivityResultStatus::Success {
                // Only process mints
                if let WalletActivityKind::Mint(mint_stage) = &activity.kind {
                    // Extract the provider from the mint stage
                    let provider = match mint_stage {
                        WalletActivityMintStage::Init(init) => &init.provider,
                        WalletActivityMintStage::Deposit(deposit) => &deposit.provider,
                        WalletActivityMintStage::Proof(proof) => &proof.init_data.provider,
                        WalletActivityMintStage::Ethereum(ethereum) => &ethereum.init_data.provider,
                        WalletActivityMintStage::Rollup(rollup) => &rollup.init_data.provider,
                        WalletActivityMintStage::Success(success) => &success.init_data.provider,
                    };

                    // Check if provider exists at all (as that was used to determine if derived private key should be used)
                    if provider.is_some() {
                        failed_mints.push((activity_id.clone(), activity));
                    }
                }
            }
        }

        failed_mints
    }
}
