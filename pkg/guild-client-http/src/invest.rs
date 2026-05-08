use guild_interface::invest::YieldPosition;

use crate::GuildClientHttp;

/// Invest / yield RPC error
pub type Error = client_http::Error<guild_interface::invest::Error>;

impl GuildClientHttp {
    /// Fetch aggregate invested / withdrawn totals for the authenticated user.
    pub async fn get_yield_position(&self) -> Result<YieldPosition, Error> {
        self.http_client
            .get("/wallets/me/invest/position")
            .auth()
            .exec()
            .await?
            .to_value()
            .await
    }
}
