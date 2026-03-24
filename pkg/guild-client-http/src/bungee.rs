use client_http::HttpBody;
use guild_interface::bungee::{
    self, GetQuoteInput, GetQuoteOutput, GetStatusInput, GetStatusOutput, GetTokenListOutput,
};

use crate::GuildClientHttp;

/// Bungee client methods
pub type Error = client_http::Error<bungee::Error>;

impl GuildClientHttp {
    /// Get a Bungee (Inbox) quote from Guild
    pub async fn get_bungee_quote(&self, request: &GetQuoteInput) -> Result<GetQuoteOutput, Error> {
        self.http_client
            .post(
                "/crypto/bungee/quote",
                Some(HttpBody::json(request.clone())),
            )
            .exec()
            .await?
            .to_value()
            .await
    }

    /// Retrieve the cached Bungee token list, scoped to the provided chain ids
    pub async fn get_bungee_token_list(
        &self,
        chain_ids: &[u128],
    ) -> Result<GetTokenListOutput, Error> {
        let snapshot = if let Some(cached) = self.token_list_cache.lock().clone() {
            cached
        } else {
            let tokens: GetTokenListOutput = self
                .http_client
                .get("/crypto/bungee/tokens")
                .exec()
                .await?
                .to_value()
                .await?;
            *self.token_list_cache.lock() = Some(tokens.clone());
            tokens
        };

        if chain_ids.is_empty() {
            return Ok(snapshot);
        }

        let mut filtered = std::collections::BTreeMap::new();
        for &chain_id in chain_ids {
            if let Some(tokens) = snapshot.tokens.get(&chain_id) {
                filtered.insert(chain_id, tokens.clone());
            }
        }

        Ok(GetTokenListOutput { tokens: filtered })
    }

    /// Fetch the latest status for a submitted Bungee bridge
    pub async fn get_bungee_status(
        &self,
        request: &GetStatusInput,
    ) -> Result<GetStatusOutput, Error> {
        let query = request.to_query_pairs().map_err(Error::Rpc)?;

        self.http_client
            .get("/crypto/bungee/status")
            .query(query)
            .exec()
            .await?
            .to_value()
            .await
    }
}
