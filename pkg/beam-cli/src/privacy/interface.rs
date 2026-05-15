// lint-long-file-override allow-max-lines=300
#![allow(
    dead_code,
    reason = "beam privacy interface is the neutral adapter contract for compatible networks"
)]

use async_trait::async_trait;
use contextful::ResultContextExt;
use contracts::Address;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    privacy::{PrivacyContext, address_to_bytes, hex32, validate_privacy_readiness},
    privacy_config::PrivacyProfile,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateAddressExport {
    pub evm_address: String,
    pub private_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateBalanceSnapshot {
    pub checkpoint_block: u64,
    pub spendable_atomic: String,
    pub token: Address,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomingTransferSummary {
    pub block_number: u64,
    pub commitment: String,
    pub status: String,
    pub tx_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyOperationReceipt {
    pub block_number: Option<u64>,
    pub operation: String,
    pub state: String,
    pub tx_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyOperationRequest {
    Mint {
        amount_atomic: String,
        token: Address,
    },
    Burn {
        amount_atomic: String,
        recipient: Address,
        token: Address,
    },
    Send {
        amount_atomic: String,
        memo: Option<[u8; 32]>,
        recipient_private_address: String,
        token: Address,
    },
    Claim {
        source: String,
    },
    EphemeralSend {
        amount_atomic: String,
        memo: Option<[u8; 32]>,
        token: Address,
    },
}

#[async_trait]
pub trait BeamPrivacyClient: Send + Sync {
    fn profile(&self) -> &PrivacyProfile;

    fn private_address(&self) -> PrivateAddressExport;

    async fn validate(&self) -> Result<()>;

    async fn balance(&self, token: Address) -> Result<PrivateBalanceSnapshot>;

    async fn incoming(
        &self,
        from_block: u64,
        to_block: Option<u64>,
        include_spent: bool,
    ) -> Result<Vec<IncomingTransferSummary>>;

    async fn submit(&self, request: PrivacyOperationRequest) -> Result<PrivacyOperationReceipt>;
}

#[async_trait]
impl BeamPrivacyClient for PrivacyContext {
    fn profile(&self) -> &PrivacyProfile {
        &self.profile
    }

    fn private_address(&self) -> PrivateAddressExport {
        PrivateAddressExport {
            evm_address: format!("{:#x}", self.evm_address),
            private_address: self.privacy_address_hex(),
        }
    }

    async fn validate(&self) -> Result<()> {
        validate_privacy_readiness(&self.client, &self.chain, &self.profile).await
    }

    async fn balance(&self, token: Address) -> Result<PrivateBalanceSnapshot> {
        let balance = self
            .client
            .privacy()
            .balances()
            .get(payy_evm_client::OwnedNoteGetParams {
                privacy_account: self.account.clone(),
                token: address_to_bytes(token),
            })
            .await
            .context("read beam privacy interface balance")?;
        let spendable = balance
            .balance
            .map_or(element::Element::ZERO, |balance| balance.spendable);
        Ok(PrivateBalanceSnapshot {
            checkpoint_block: balance.owned_note_state.checked_block,
            spendable_atomic: contracts::U256::from_big_endian(&spendable.to_be_bytes())
                .to_string(),
            token,
        })
    }

    async fn incoming(
        &self,
        from_block: u64,
        to_block: Option<u64>,
        include_spent: bool,
    ) -> Result<Vec<IncomingTransferSummary>> {
        let notes = self
            .client
            .privacy()
            .incoming()
            .list(payy_evm_client::IncomingListParams {
                privacy_account: self.account.clone(),
                privacy_address_prefix: None,
                from_block,
                to_block,
                include_spent,
                poll_interval_ms: None,
            })
            .await
            .context("list beam privacy interface incoming transfers")?;
        Ok(notes
            .into_iter()
            .map(|note| IncomingTransferSummary {
                block_number: note.source_position.block_number,
                commitment: hex32(&note.commitment),
                status: format!("{:?}", note.status).to_ascii_lowercase(),
                tx_hash: hex32(&note.source_tx_hash),
            })
            .collect())
    }

    async fn submit(&self, request: PrivacyOperationRequest) -> Result<PrivacyOperationReceipt> {
        let feature = match request {
            PrivacyOperationRequest::Mint { .. } => "mint",
            PrivacyOperationRequest::Burn { .. } => "burn",
            PrivacyOperationRequest::Send { .. } => "send",
            PrivacyOperationRequest::Claim { .. } => "claim",
            PrivacyOperationRequest::EphemeralSend { .. } => "ephemeral-send",
        };
        Err(Error::PrivacyFeatureUnsupported {
            chain: self.chain.entry.key.clone(),
            feature: format!("interface-submit-{feature}"),
        })
    }
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MockBeamPrivacyClient {
    pub address: PrivateAddressExport,
    pub profile: PrivacyProfile,
}

#[cfg(test)]
#[async_trait]
impl BeamPrivacyClient for MockBeamPrivacyClient {
    fn profile(&self) -> &PrivacyProfile {
        &self.profile
    }

    fn private_address(&self) -> PrivateAddressExport {
        self.address.clone()
    }

    async fn validate(&self) -> Result<()> {
        Ok(())
    }

    async fn balance(&self, token: Address) -> Result<PrivateBalanceSnapshot> {
        Ok(PrivateBalanceSnapshot {
            checkpoint_block: 0,
            spendable_atomic: "0".to_string(),
            token,
        })
    }

    async fn incoming(
        &self,
        from_block: u64,
        to_block: Option<u64>,
        include_spent: bool,
    ) -> Result<Vec<IncomingTransferSummary>> {
        let _ = (from_block, to_block, include_spent);
        Ok(vec![IncomingTransferSummary {
            block_number: 0,
            commitment: format!("0x{}", "00".repeat(32)),
            status: "claimable".to_string(),
            tx_hash: format!("0x{}", "00".repeat(32)),
        }])
    }

    async fn submit(&self, request: PrivacyOperationRequest) -> Result<PrivacyOperationReceipt> {
        let operation = match request {
            PrivacyOperationRequest::Mint { .. } => "mint",
            PrivacyOperationRequest::Burn { .. } => "burn",
            PrivacyOperationRequest::Send { .. } => "send",
            PrivacyOperationRequest::Claim { .. } => "claim",
            PrivacyOperationRequest::EphemeralSend { .. } => "ephemeral-send",
        };
        Ok(PrivacyOperationReceipt {
            block_number: Some(0),
            operation: operation.to_string(),
            state: "confirmed".to_string(),
            tx_hash: format!("0x{}", "00".repeat(32)),
        })
    }
}
