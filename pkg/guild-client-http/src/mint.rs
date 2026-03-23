use client_http::HttpBody;
use guild_interface::mint::{self, MintRequest, MintResponse};

use crate::GuildClientHttp;

/// Mint error
pub type Error = client_http::Error<mint::Error>;

impl GuildClientHttp {
    /// Send mint request to Guild
    pub async fn mint(&self, request: &MintRequest) -> Result<MintResponse, Error> {
        self.http_client
            .post("/mint/signed", Some(HttpBody::json(request.clone())))
            .exec()
            .await?
            .to_value()
            .await
    }
}
