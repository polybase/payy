use async_trait::async_trait;
use test_spy::spy_mock;
use uuid::Uuid;

use crate::error::Result;

use super::{
    CreateTransactionRequest, LimitQuery, ListRampsTransactionsQuery, RampTransaction,
    RemainingLimits, Transaction, UpdateTransactionRequest,
};

#[spy_mock]
#[async_trait]
pub trait TransactionsInterface: Send + Sync {
    async fn create_transaction(
        &self,
        wallet_id: Uuid,
        request: CreateTransactionRequest,
    ) -> Result<Transaction>;

    async fn list_transactions(
        &self,
        wallet_id: Uuid,
        query: ListRampsTransactionsQuery,
    ) -> Result<Vec<RampTransaction>>;

    async fn get_transaction(&self, wallet_id: Uuid, transaction_id: Uuid) -> Result<Transaction>;

    async fn update_transaction(
        &self,
        wallet_id: Uuid,
        transaction_id: Uuid,
        request: UpdateTransactionRequest,
    ) -> Result<Transaction>;

    async fn get_limits(&self, wallet_id: Uuid, query: LimitQuery) -> Result<RemainingLimits>;
}
