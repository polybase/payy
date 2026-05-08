use async_trait::async_trait;
use unimock::unimock;
use uuid::Uuid;

use crate::error::Result;

use super::{
    CreateTransactionRequest, FundTransactionRequest, LimitQuery, ListRampsTransactionsQuery,
    RampTransaction, RemainingLimits, Transaction, UpdateTransactionRequest,
};

#[unimock(api = TransactionsInterfaceMock)]
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

    async fn fund_transaction(
        &self,
        wallet_id: Uuid,
        transaction_id: Uuid,
        request: FundTransactionRequest,
    ) -> Result<Transaction>;

    async fn get_limits(&self, wallet_id: Uuid, query: LimitQuery) -> Result<RemainingLimits>;
}
