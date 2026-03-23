use chrono::{DateTime, Utc};
use client_http::{ClientResponse, NoRpcError, serde_to_query_params};
use client_http_longpoll::{LongPoll, LongPollPoller};
use guild_interface::ramps::{ListRampsTransactionsQuery, RampTransaction};
use ramps_interface::transaction::Transaction;
use rpc::longpoll::PollData;
use uuid::Uuid;

use crate::GuildClientHttp;

/// Note error
pub type Error = client_http::Error<NoRpcError>;

impl GuildClientHttp {
    /// Get a list of the users notes
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

    /// Get a list of notes with long poll
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
