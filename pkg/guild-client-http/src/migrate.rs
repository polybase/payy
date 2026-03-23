use client_http::{HttpBody, NoRpcError};
use guild_interface::migrate::{MigrationNotesRequest, MigrationNotesResponse};

use crate::GuildClientHttp;

/// Mint error
pub type Error = client_http::Error<NoRpcError>;

impl GuildClientHttp {
    /// Send mint request to Guild
    pub async fn migrate_notes(
        &self,
        request: &MigrationNotesRequest,
    ) -> Result<MigrationNotesResponse, Error> {
        self.http_client
            .post("/migrate/notes", Some(HttpBody::json(request.clone())))
            .auth()
            .exec()
            .await?
            .to_value()
            .await
    }
}
