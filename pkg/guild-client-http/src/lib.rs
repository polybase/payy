#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! HTTPS client for guild

use auth::GuildClientHttpAuth;
use client_http::ClientHttp;
use element::Element;
use guild_interface::bungee::GetTokenListOutput;
pub use reqwest::Url;
use reqwest::header::HeaderMap;
use std::sync::Arc;
use zk_circuits::BbBackend;

use http_interface::HttpClient;
use parking_lot::Mutex;
/// Across client methods
pub mod across;
/// Bungee client methods
pub mod bungee;

mod auth;
/// EIP-7702 client methods
pub mod eip7702;
mod error;
/// Migration methods
pub mod migrate;
/// Mint client methods
pub mod mint;
/// Note client methods
pub mod note;
/// Ramps methods
pub mod ramps;
/// Registry client methods
pub mod registry;
/// Support methods
pub mod support;
/// Wallet methods
pub mod wallet;
/// Wallet activity methods
pub mod wallet_activity;
/// Wallet activity notes methods
pub mod wallet_notes;

pub use error::{Error, Result};

/// Guild client for interacting with Guild server over HTTPS
#[derive(Clone)]
pub struct GuildClientHttp {
    pub(crate) http_client: Arc<dyn HttpClient>,
    pub(crate) token_list_cache: Arc<Mutex<Option<GetTokenListOutput>>>,
}

impl GuildClientHttp {
    /// Create a new guild https client backed by the default HTTP implementation.
    #[must_use]
    pub fn new(base_url: Url, private_key: Element, bb_backend: Arc<dyn BbBackend>) -> Self {
        let auth = GuildClientHttpAuth::new(base_url.clone(), private_key, bb_backend);
        Self::with_http_client(ClientHttp::new(base_url, HeaderMap::default(), auth))
    }

    /// Construct a guild client with a pre-configured HTTP client implementation.
    #[must_use]
    pub fn with_http_client<C>(http_client: C) -> Self
    where
        C: HttpClient + 'static,
    {
        Self::with_dyn_http_client(Arc::new(http_client))
    }

    /// Construct a guild client backed by a trait-object HTTP client.
    #[must_use]
    pub fn with_dyn_http_client(http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            http_client,
            token_list_cache: Arc::new(Mutex::new(None)),
        }
    }
}
