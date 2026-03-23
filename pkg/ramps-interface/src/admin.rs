use async_trait::async_trait;
use kyc::KycUpdateRequired;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use test_spy::spy_mock;
use uuid::Uuid;

use crate::account::Account;
use crate::error::Result;
use crate::provider::Provider;
use crate::transaction::{RampTransaction, Transaction};

/// Authorization metadata extracted from admin HTTP requests.
#[derive(Debug, Clone, Default)]
pub struct AdminAuthContext {
    /// Raw `Authorization` header value, if supplied by the client.
    pub authorization: Option<String>,
}

/// Request payload for fulfilling unfunded transactions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FulfillUnfundedRequest {
    pub address: String,
    pub provider: Option<Provider>,
}

/// Response payload summarising the outcome of an unfunded fulfilment run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FulfillUnfundedResult {
    pub total_amount: u64,
    pub failed: Vec<RampTransaction>,
    pub total_transactions: u32,
}

/// Response payload containing data from an individual fulfilment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillTransactionResult {
    pub transaction: RampTransaction,
    pub fulfilled_amount: u64,
}

impl Default for FulfillTransactionResult {
    fn default() -> Self {
        Self {
            transaction: RampTransaction::from(Transaction::default()),
            fulfilled_amount: 0,
        }
    }
}

/// Request payload for rejecting an account with an optional reason.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RejectAccountRequest {
    pub rejection_reason: Option<KycUpdateRequired>,
}

/// Request payload for updating required KYC fields on an account.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateRequiredRequest {
    pub update_required: KycUpdateRequired,
}

/// Request payload for updating account metadata.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateMetadataRequest {
    pub metadata: Value,
}

/// Response payload for admin-triggered settlement.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SettleTransactionResult {
    pub success: bool,
    pub message: String,
    pub transaction_id: Uuid,
    pub wallet_id: Uuid,
}

/// Interface describing admin-centric ramps operations.
#[spy_mock]
#[async_trait]
pub trait AdminInterface: Send + Sync {
    /// Fulfil any unfunded transactions for a wallet address.
    async fn fulfill_unfunded(
        &self,
        auth: AdminAuthContext,
        request: FulfillUnfundedRequest,
    ) -> Result<FulfillUnfundedResult>;

    /// Fulfil a specific transaction by identifier.
    async fn fulfill_transaction(
        &self,
        auth: AdminAuthContext,
        transaction_id: Uuid,
    ) -> Result<FulfillTransactionResult>;

    /// Approve an account by identifier.
    async fn approve_account(
        &self,
        auth: AdminAuthContext,
        account_id: Uuid,
        ip_address: String,
    ) -> Result<Account>;

    /// Reject an account by identifier with an optional reason.
    async fn reject_account(
        &self,
        auth: AdminAuthContext,
        account_id: Uuid,
        rejection_reason: Option<KycUpdateRequired>,
    ) -> Result<Account>;

    /// Update required fields on an account.
    async fn update_required(
        &self,
        auth: AdminAuthContext,
        account_id: Uuid,
        update_required: KycUpdateRequired,
    ) -> Result<Account>;

    /// Update account metadata by identifier.
    async fn update_account_metadata(
        &self,
        auth: AdminAuthContext,
        account_id: Uuid,
        metadata: Value,
    ) -> Result<Account>;

    /// Trigger settlement for a transaction.
    async fn settle_transaction(
        &self,
        auth: AdminAuthContext,
        transaction_id: Uuid,
    ) -> Result<SettleTransactionResult>;
}
