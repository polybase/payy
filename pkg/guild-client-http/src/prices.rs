use client_http::serde_to_query_params;
use currency::Currency;
use guild_interface::prices::GetTokenPriceQuery;
use price_cache_interface::{TokenIdentifier, TokenPrice};

use crate::GuildClientHttp;

/// Prices RPC error.
pub type Error = client_http::Error<guild_interface::prices::Error>;

impl GuildClientHttp {
    /// Fetch a token price from guild.
    pub async fn get_token_price(
        &self,
        token: &TokenIdentifier,
        currency: Currency,
    ) -> Result<TokenPrice, Error> {
        let query = serde_to_query_params(&GetTokenPriceQuery { currency });
        let request = match token {
            TokenIdentifier::Symbol { symbol } => self
                .http_client
                .get(&format!("/prices/symbol/{symbol}"))
                .query(query),
            TokenIdentifier::Address { network, address } => self
                .http_client
                .get(&format!("/prices/address/{network}/{address}"))
                .query(query),
        };

        request.auth().exec().await?.to_value().await
    }
}
