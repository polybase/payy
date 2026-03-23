use async_trait::async_trait;
use chrono::{DateTime, Utc};
use client_http::{ClientResponse, NoRpcError, serde_to_query_params};
use client_http_longpoll::{LongPoll, LongPollPoller};
use guild_interface::wallet::{GetWalletQuery, Wallet};
use rpc::longpoll::PollData;

use crate::GuildClientHttp;

/// Wallet error
pub type Error = client_http::Error<NoRpcError>;

impl GuildClientHttp {
    /// Get wallet information
    pub async fn get_wallet(&self) -> Result<Wallet, Error> {
        self.get_wallet_response(&GetWalletQuery::default())
            .await?
            .to_value()
            .await
    }

    /// Get wallet response with query parameters
    async fn get_wallet_response(&self, query: &GetWalletQuery) -> Result<ClientResponse, Error> {
        self.http_client
            .get("/wallets/me")
            .query(serde_to_query_params(query))
            .auth()
            .exec()
            .await
    }

    /// Get wallet with long poll
    #[must_use]
    pub fn get_wallet_long_poll(&self) -> LongPoll<WalletLongPoll> {
        LongPoll::new(WalletLongPoll {
            client: self.clone(),
        })
    }
}

/// Long poll poller for wallet updates
pub struct WalletLongPoll {
    client: GuildClientHttp,
}

#[async_trait]
impl LongPollPoller for WalletLongPoll {
    type Error = Error;
    type T = Wallet;

    async fn poll(
        &self,
        last_modified: Option<DateTime<Utc>>,
    ) -> Result<PollData<Self::T>, Self::Error> {
        self.client
            .get_wallet_response(&GetWalletQuery {
                after: last_modified.and_then(|lm| u64::try_from(lm.timestamp_micros()).ok()),
                wait: Some(60),
            })
            .await?
            .to_long_poll()
            .await
    }
}
