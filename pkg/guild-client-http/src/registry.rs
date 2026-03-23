use chrono::{DateTime, Utc};
use client_http::{ClientResponse, HttpBody, serde_to_query_params};
use client_http_longpoll::{LongPoll, LongPollPoller};
use element::Element;
use guild_interface::registry::{
    self, CreateRegistryNoteInput, ListRegistryNotesQuery, RegistryNote,
};
use rpc::longpoll::PollData;

use crate::GuildClientHttp;

/// Registry error
pub type Error = client_http::Error<registry::Error>;

impl GuildClientHttp {
    /// Get a list of the registry notes for the users public key
    async fn list_registry_notes_response(
        &self,
        public_key: Element,
        query: &ListRegistryNotesQuery,
    ) -> Result<ClientResponse, Error> {
        self.http_client
            .get(format!("/registry/notes/{}", &public_key.to_string()).as_str())
            .query(serde_to_query_params(query))
            .auth()
            .exec()
            .await
    }

    /// Get a list of registry notes with long poll
    #[must_use]
    pub fn list_registry_notes_long_poll(
        &self,
        public_key: Element,
        query: &ListRegistryNotesQuery,
    ) -> LongPoll<RegistryNotesLongPoll> {
        LongPoll::new(RegistryNotesLongPoll {
            client: self.clone(),
            query: query.clone(),
            public_key,
        })
    }

    /// Send add to encrypted request
    pub async fn add_registry_entry(
        &self,
        request: &CreateRegistryNoteInput,
    ) -> Result<RegistryNote, Error> {
        self.http_client
            .post("/registry/notes", Some(HttpBody::json(request.clone())))
            .auth()
            .exec()
            .await?
            .to_value()
            .await
    }
}

/// Registry notes long poll
pub struct RegistryNotesLongPoll {
    client: GuildClientHttp,
    public_key: Element,
    query: ListRegistryNotesQuery,
}

#[async_trait::async_trait]
impl LongPollPoller for RegistryNotesLongPoll {
    type Error = Error;
    type T = Vec<RegistryNote>;

    async fn poll(
        &self,
        last_modified: Option<DateTime<Utc>>,
    ) -> Result<PollData<Self::T>, Self::Error> {
        self.client
            .list_registry_notes_response(
                self.public_key,
                &ListRegistryNotesQuery {
                    after: last_modified.and_then(|lm| u64::try_from(lm.timestamp_micros()).ok()),
                    wait: Some(60),
                    ..self.query
                },
            )
            .await?
            .to_long_poll()
            .await
    }
}
