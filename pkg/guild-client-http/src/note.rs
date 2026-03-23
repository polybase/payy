use chrono::{DateTime, Utc};
use client_http::{ClientResponse, serde_to_query_params};
use client_http_longpoll::{LongPoll, LongPollPoller};
use guild_interface::notes::{
    create::CreateNoteInput, list::ListNotesQuery, note::Note, request::NoteRequestInput,
};
use rpc::longpoll::PollData;

use crate::GuildClientHttp;
use client_http::HttpBody;

/// Note error
pub type Error = client_http::Error<guild_interface::notes::create::Error>;

impl GuildClientHttp {
    /// Create a new note
    pub async fn post_notes(&self, input: &CreateNoteInput) -> Result<Note, Error> {
        self.http_client
            .post("/notes", Some(HttpBody::json(input.clone())))
            .auth()
            .exec()
            .await?
            .to_value()
            .await
    }

    /// Request a note
    pub async fn post_notes_request(&self, input: &NoteRequestInput) -> Result<Note, Error> {
        self.http_client
            .post("/notes/request", Some(HttpBody::json(input.clone())))
            .auth()
            .exec()
            .await?
            .to_value()
            .await
    }

    /// Get a list of the users notes
    async fn list_notes_response(&self, query: &ListNotesQuery) -> Result<ClientResponse, Error> {
        self.http_client
            .get("/notes")
            .query(serde_to_query_params(query))
            .auth()
            .exec()
            .await
    }

    /// Get a list of notes with long poll
    #[must_use]
    pub fn list_notes_long_poll(&self, query: &ListNotesQuery) -> LongPoll<NotesLongPoll> {
        LongPoll::new(NotesLongPoll {
            client: self.clone(),
            query: query.clone(),
        })
    }
}

/// Long poll handler for notes
///
/// This struct encapsulates the client and query parameters needed to perform
/// long polling operations on notes, allowing for real-time updates when notes change.
pub struct NotesLongPoll {
    client: GuildClientHttp,
    query: ListNotesQuery,
}

#[async_trait::async_trait]
impl LongPollPoller for NotesLongPoll {
    type Error = Error;
    type T = Vec<Note>;

    async fn poll(
        &self,
        last_modified: Option<DateTime<Utc>>,
    ) -> Result<PollData<Self::T>, Self::Error> {
        self.client
            .list_notes_response(&ListNotesQuery {
                after: last_modified.and_then(|lm| u64::try_from(lm.timestamp_micros()).ok()),
                wait: Some(60),
                ..self.query
            })
            .await?
            .to_long_poll()
            .await
    }
}
