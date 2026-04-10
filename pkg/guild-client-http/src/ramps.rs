use chrono::{DateTime, Utc};
use client_http::{ClientResponse, HttpBody, NoRpcError, serde_to_query_params};
use client_http_longpoll::{LongPoll, LongPollPoller};
use guild_interface::ramps::{ListRampsTransactionsQuery, RampTransaction};
use ramps_interface::transaction::{CreateTransactionRequest, FundTransactionRequest, Transaction};
use rpc::longpoll::PollData;
use uuid::Uuid;

use crate::GuildClientHttp;

/// Note error
pub type Error = client_http::Error<NoRpcError>;
/// Typed RPC error for ramps transaction creation.
pub type CreateError = client_http::Error<ramps_interface::Error>;
/// Typed RPC error for ramps transaction funding.
pub type FundError = client_http::Error<ramps_interface::Error>;

impl GuildClientHttp {
    /// Create a new ramps transaction.
    pub async fn create_transaction(
        &self,
        request: &CreateTransactionRequest,
    ) -> Result<RampTransaction, CreateError> {
        self.http_client
            .post("/ramps/transactions", Some(HttpBody::json(request.clone())))
            .auth()
            .exec()
            .await?
            .to_value::<Transaction, ramps_interface::Error>()
            .await
            .map(RampTransaction::from)
    }

    /// Fund an existing ramps transaction.
    pub async fn fund_transaction(
        &self,
        transaction_id: Uuid,
        request: &FundTransactionRequest,
    ) -> Result<RampTransaction, FundError> {
        self.http_client
            .post(
                &format!("/ramps/transactions/{transaction_id}/fund"),
                Some(HttpBody::json(request.clone())),
            )
            .auth()
            .exec()
            .await?
            .to_value::<Transaction, ramps_interface::Error>()
            .await
            .map(RampTransaction::from)
    }

    /// Get a list of the user's ramps transactions.
    async fn list_ramps_transactions_response(
        &self,
        query: &ListRampsTransactionsQuery,
    ) -> Result<ClientResponse, Error> {
        self.http_client
            .get("/ramps/transactions")
            .query(serde_to_query_params(query))
            .auth()
            .exec()
            .await
    }

    /// Get a list of ramps transactions with long poll.
    #[must_use]
    pub fn list_ramps_transactions_long_poll(
        &self,
        query: &ListRampsTransactionsQuery,
    ) -> LongPoll<RampTransactionLongPoll> {
        // TODO: attach long poll to client
        LongPoll::new(RampTransactionLongPoll {
            client: self.clone(),
            query: query.clone(),
        })
    }

    /// Get a single ramps transaction by id
    pub async fn get_ramp_transaction(&self, id: Uuid) -> Result<RampTransaction, Error> {
        self.http_client
            .get(&format!("/ramps/transactions/{id}"))
            .auth()
            .exec()
            .await?
            .to_value::<Transaction, NoRpcError>()
            .await
            .map(RampTransaction::from)
    }
}

/// Polls for ramps transactions
pub struct RampTransactionLongPoll {
    client: GuildClientHttp,
    query: ListRampsTransactionsQuery,
}

#[async_trait::async_trait]
impl LongPollPoller for RampTransactionLongPoll {
    type Error = Error;
    type T = Vec<RampTransaction>;

    async fn poll(
        &self,
        last_modified: Option<DateTime<Utc>>,
    ) -> Result<PollData<Self::T>, Self::Error> {
        self.client
            .list_ramps_transactions_response(&ListRampsTransactionsQuery {
                after: last_modified.and_then(|lm| u64::try_from(lm.timestamp_micros()).ok()),
                wait: Some(60),
                ..self.query.clone()
            })
            .await?
            .to_long_poll()
            .await
    }
}
