use client_http::{ClientResponse, HttpBody, serde_to_query_params};
use guild_interface::wallet_activity::{
    WalletActivity, WalletActivityListQuery, WalletActivityUpsert,
};
use uuid::Uuid;

use crate::GuildClientHttp;

/// Error for wallet activity rpc
pub type Error = client_http::Error<guild_interface::wallet_activity::Error>;

impl GuildClientHttp {
    /// Send wallet activity upsert request to Guild
    pub async fn upsert_wallet_activity(
        &self,
        wallet_activity: &WalletActivityUpsert,
    ) -> Result<WalletActivity, Error> {
        self.http_client
            .post(
                "/wallets/me/activity",
                Some(HttpBody::json(wallet_activity.clone())),
            )
            .auth()
            .exec()
            .await?
            .to_value()
            .await
    }

    /// Lists wallet activity for the authenticated user, with optional filters
    pub async fn list_wallet_activity(
        &self,
        query: Option<WalletActivityListQuery>,
    ) -> Result<Vec<WalletActivity>, Error> {
        let mut req = self.http_client.get("/wallets/me/activity").auth();

        if let Some(query) = query {
            req = req.query(serde_to_query_params(&query));
        }

        req.exec().await?.to_value().await
    }

    /// Get a specific activity
    pub async fn get_wallet_activity(&self, id: &Uuid) -> Result<Option<WalletActivity>, Error> {
        let res: Result<ClientResponse, Error> = self
            .http_client
            .get(&format!("/wallets/me/activity/{id}"))
            .auth()
            .exec()
            .await;

        let res = match res {
            Ok(res) => res,
            Err(client_http::Error::Rpc(
                guild_interface::wallet_activity::Error::ActivityNotFound,
            )) => {
                return Ok(None);
            }
            Err(err) => return Err(err),
        };

        let val: WalletActivity = res.to_value().await?;

        Ok(Some(val))
    }
}
