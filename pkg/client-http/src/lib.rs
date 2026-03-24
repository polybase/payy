#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Base HTTPS client designed to be extended by a specific service client struct.

use std::{sync::Arc, time::Duration};

use reqwest::{Method, Url, header::HeaderMap};
use tracing::warn;

mod builder;
mod error;
mod request;
mod response;
mod util;

pub use error::{Error, NoRpcError, Result};
pub use http_interface::{
    AuthError, ClientHttpAuth, HttpClient, HttpRequestBuilder, builder::HttpBody,
};
pub use response::ClientResponse;
pub use util::serde_to_query_params;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// HTTPS client for making calls to servers over HTTPS.
#[derive(Clone)]
pub struct ClientHttp {
    pub(crate) http_client: reqwest::Client,
    pub(crate) base_url: Arc<Url>,
    pub(crate) headers: HeaderMap,
    pub(crate) auth: Arc<dyn ClientHttpAuth>,
}

impl ClientHttp {
    /// Create a new client with the provided auth implementation.
    #[must_use]
    pub fn new<A>(base_url: Url, headers: HeaderMap, auth: A) -> Self
    where
        A: ClientHttpAuth + 'static,
    {
        let http_client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .unwrap_or_else(|err| {
                warn!(
                    error = %err,
                    "failed to build reqwest client with connect timeout; using default"
                );
                reqwest::Client::new()
            });

        Self {
            http_client,
            base_url: Arc::new(base_url),
            headers,
            auth: Arc::new(auth),
        }
    }

    fn request_builder(
        &self,
        path: &str,
        method: Method,
        body: Option<HttpBody>,
    ) -> builder::RequestBuilder {
        builder::RequestBuilder::new(
            self.clone(),
            path.to_owned(),
            method,
            body.map(|b| b.applier()),
        )
    }

    /// Configure a GET request.
    #[must_use]
    pub fn get(&self, path: &str) -> builder::RequestBuilder {
        self.request_builder(path, Method::GET, None)
    }

    /// Configure a POST request.
    #[must_use]
    pub fn post(&self, path: &str, body: Option<HttpBody>) -> builder::RequestBuilder {
        self.request_builder(path, Method::POST, body)
    }

    /// Configure a DELETE request.
    #[must_use]
    pub fn delete(&self, path: &str, body: Option<HttpBody>) -> builder::RequestBuilder {
        self.request_builder(path, Method::DELETE, body)
    }

    /// Configure a PUT request.
    #[must_use]
    pub fn put(&self, path: &str, body: Option<HttpBody>) -> builder::RequestBuilder {
        self.request_builder(path, Method::PUT, body)
    }

    /// Configure a PATCH request.
    #[must_use]
    pub fn patch(&self, path: &str, body: Option<HttpBody>) -> builder::RequestBuilder {
        self.request_builder(path, Method::PATCH, body)
    }
}

/// Default auth impl is no auth.
#[derive(Default, Clone)]
pub struct NoAuth;

#[async_trait::async_trait]
impl ClientHttpAuth for NoAuth {
    async fn get_auth(&self) -> std::result::Result<HeaderMap, AuthError> {
        Ok(HeaderMap::new())
    }

    async fn refresh_auth(&self) -> std::result::Result<(), AuthError> {
        Ok(())
    }
}

impl HttpClient for ClientHttp {
    fn get(&self, path: &str) -> HttpRequestBuilder {
        HttpRequestBuilder::new(Box::new(ClientHttp::get(self, path)))
    }

    fn post(&self, path: &str, body: Option<HttpBody>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(Box::new(self.request_builder(path, Method::POST, body)))
    }

    fn delete(&self, path: &str, body: Option<HttpBody>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(Box::new(self.request_builder(path, Method::DELETE, body)))
    }

    fn put(&self, path: &str, body: Option<HttpBody>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(Box::new(self.request_builder(path, Method::PUT, body)))
    }

    fn patch(&self, path: &str, body: Option<HttpBody>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(Box::new(self.request_builder(path, Method::PATCH, body)))
    }
}
