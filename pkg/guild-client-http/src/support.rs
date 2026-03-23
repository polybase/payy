use chrono::{DateTime, Utc};
use client_http::{ClientResponse, NoRpcError, serde_to_query_params};
use client_http_longpoll::{LongPoll, LongPollPoller};
use guild_interface::support::{ListSupportIssuesQuery, SupportIssue};
use rpc::longpoll::PollData;

use crate::GuildClientHttp;

/// Support error
pub type Error = client_http::Error<NoRpcError>;

impl GuildClientHttp {
    /// Get a list of the support issues for the user
    async fn list_support_issues_response(
        &self,
        query: &ListSupportIssuesQuery,
    ) -> Result<ClientResponse, Error> {
        self.http_client
            .get("/support/issues")
            .query(serde_to_query_params(query))
            .auth()
            .exec()
            .await
    }

    /// Get a list of support issues with long poll
    #[must_use]
    pub fn list_support_issues_long_poll(
        &self,
        query: &ListSupportIssuesQuery,
    ) -> LongPoll<SupportIssuesLongPoll> {
        LongPoll::new(SupportIssuesLongPoll {
            client: self.clone(),
            query: query.clone(),
        })
    }
}

/// Support issues long poll
pub struct SupportIssuesLongPoll {
    client: GuildClientHttp,
    query: ListSupportIssuesQuery,
}

#[async_trait::async_trait]
impl LongPollPoller for SupportIssuesLongPoll {
    type Error = Error;
    type T = Vec<SupportIssue>;

    async fn poll(
        &self,
        last_modified: Option<DateTime<Utc>>,
    ) -> Result<PollData<Self::T>, Self::Error> {
        let res = self
            .client
            .list_support_issues_response(&ListSupportIssuesQuery {
                after: last_modified.and_then(|lm| u64::try_from(lm.timestamp_micros()).ok()),
                wait: Some(60),
                ..self.query
            })
            .await?
            .to_long_poll()
            .await?;

        Ok(res)
    }
}
