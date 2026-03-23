use client_http::HttpBody;
use guild_interface::across::{
    self, DepositV3WithAuthorizationInput, DepositV3WithAuthorizationOutput,
};

use crate::GuildClientHttp;

/// Across error
pub type Error = client_http::Error<across::Error>;

impl GuildClientHttp {
    /// Send across deposit request to Guild
    pub async fn across_deposit(
        &self,
        request: &DepositV3WithAuthorizationInput,
    ) -> Result<DepositV3WithAuthorizationOutput, Error> {
        self.http_client
            .post(
                "/crypto/across/deposit",
                Some(HttpBody::json(request.clone())),
            )
            .exec()
            .await?
            .to_value()
            .await
    }

    /// Get an Across quote from Guild
    pub async fn get_across_quote(
        &self,
        request: &across::GetQuoteInput,
    ) -> Result<across::GetQuoteOutput, Error> {
        self.http_client
            .post(
                "/crypto/across/quote",
                Some(HttpBody::json(request.clone())),
            )
            .exec()
            .await?
            .to_value()
            .await
    }
}
