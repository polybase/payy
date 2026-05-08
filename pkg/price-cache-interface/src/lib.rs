#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::doc_markdown)]
#![allow(missing_docs)]

mod error;
mod types;

pub use error::{Error, Result};
pub use types::{TokenIdentifier, TokenPrice};

use async_trait::async_trait;
use currency::Currency;
use unimock::unimock;

/// Shared interface for reading token prices from the price cache.
#[unimock(api = PriceCacheMock)]
#[async_trait]
pub trait PriceCache: Send + Sync {
    async fn get_price(&self, token: &TokenIdentifier, currency: Currency) -> Result<TokenPrice>;
}
