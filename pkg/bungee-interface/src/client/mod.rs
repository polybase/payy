use async_trait::async_trait;

mod error;
mod quote;
mod status;
mod token_list;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use quote::{GetQuoteInput, GetQuoteOutput};
pub use status::{
    BungeeStatusCode, GetStatusInput, GetStatusOutput, StatusEntry, StatusIdentifier,
};
pub use token_list::{GetTokenListInput, GetTokenListOutput, TokenListKind, TokenMetadata};

/// Trait for the Bungee domain service used by Guild and wallet clients.
#[async_trait]
pub trait BungeeClient: Send + Sync + 'static {
    /// Get a Bungee quote.
    async fn get_quote(&self, input: &GetQuoteInput) -> Result<GetQuoteOutput>;

    /// Fetch the Bungee token list grouped by chain id.
    async fn get_token_list(&self, input: &GetTokenListInput) -> Result<GetTokenListOutput>;

    /// Look up the status of a submitted bridge request.
    async fn get_status(&self, input: &GetStatusInput) -> Result<GetStatusOutput>;
}
