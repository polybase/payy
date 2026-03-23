use crate::{
    ClientResponse, HttpMetadata, HttpRequestBuilder,
    builder::{HttpBody, HttpRequestExecutor},
    error::{AuthError, HttpRequestExecError},
};
use reqwest::header::HeaderMap;
use std::sync::{Arc, Mutex};

/// Implement auth for the HTTP client; invoked whenever `.auth()` is used on a request.
#[async_trait::async_trait]
pub trait ClientHttpAuth: Send + Sync {
    /// Get auth headers (can be a cached version).
    async fn get_auth(&self) -> std::result::Result<HeaderMap, AuthError>;

    /// Refresh auth. Called if the auth provided by `get_auth` is invalid (usually because it
    /// needs to be refreshed).
    async fn refresh_auth(&self) -> std::result::Result<(), AuthError>;
}

/// Abstraction over HTTP client implementations to support swapping, mocking,
/// and dependency injection across crates.
#[unimock::unimock(api = HttpClientMock)]
pub trait HttpClient: Send + Sync {
    /// Configure a GET request for the provided path.
    fn get(&self, path: &str) -> HttpRequestBuilder;

    /// Configure a POST request for the provided path.
    fn post(&self, path: &str, body: Option<HttpBody>) -> HttpRequestBuilder;

    /// Configure a DELETE request for the provided path.
    fn delete(&self, path: &str, body: Option<HttpBody>) -> HttpRequestBuilder;

    /// Configure a PUT request for the provided path.
    fn put(&self, path: &str, body: Option<HttpBody>) -> HttpRequestBuilder;

    /// Configure a PATCH request for the provided path.
    fn patch(&self, path: &str, body: Option<HttpBody>) -> HttpRequestBuilder;
}

/// Simple recording HTTP client for integration tests that need to assert calls.
#[derive(Clone)]
pub struct RecordingHttpClient {
    calls: Arc<Mutex<Vec<(reqwest::Method, String)>>>,
}

impl RecordingHttpClient {
    /// Create a new recording client
    #[must_use]
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get recorded calls
    #[must_use]
    pub fn calls(&self) -> Vec<(reqwest::Method, String)> {
        self.calls.lock().unwrap().clone()
    }

    /// Record a call
    fn record_call(&self, method: reqwest::Method, path: &str) {
        self.calls.lock().unwrap().push((method, path.to_owned()));
    }
}

impl Default for RecordingHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock request builder that implements HttpRequestBuilder for testing
#[derive(Clone)]
pub struct MockRequestBuilder {
    method: reqwest::Method,
    path: String,
    headers: Option<reqwest::header::HeaderMap>,
    query: Option<Vec<(String, String)>>,
    auth: bool,
}

impl MockRequestBuilder {
    fn new(method: reqwest::Method, path: String) -> Self {
        Self {
            method,
            path,
            headers: None,
            query: None,
            auth: false,
        }
    }
}

#[async_trait::async_trait]
impl HttpRequestExecutor for MockRequestBuilder {
    fn headers(&mut self, headers: reqwest::header::HeaderMap) {
        self.headers = Some(headers);
    }

    fn query(&mut self, query: Vec<(String, String)>) {
        self.query = Some(query);
    }

    fn auth(&mut self) {
        self.auth = true;
    }

    async fn exec(self: Box<Self>) -> Result<ClientResponse, HttpRequestExecError> {
        // Default mock behavior: return server error
        Err(HttpRequestExecError::ServerError(
            "mock failure".to_string(),
            HttpMetadata {
                method: self.method,
                path: self.path,
            },
        ))
    }
}

impl HttpClient for RecordingHttpClient {
    fn get(&self, path: &str) -> HttpRequestBuilder {
        self.record_call(reqwest::Method::GET, path);
        HttpRequestBuilder::new(Box::new(MockRequestBuilder::new(
            reqwest::Method::GET,
            path.to_string(),
        )))
    }

    fn post(&self, path: &str, _body: Option<HttpBody>) -> HttpRequestBuilder {
        self.record_call(reqwest::Method::POST, path);
        HttpRequestBuilder::new(Box::new(MockRequestBuilder::new(
            reqwest::Method::POST,
            path.to_string(),
        )))
    }

    fn delete(&self, path: &str, _body: Option<HttpBody>) -> HttpRequestBuilder {
        self.record_call(reqwest::Method::DELETE, path);
        HttpRequestBuilder::new(Box::new(MockRequestBuilder::new(
            reqwest::Method::DELETE,
            path.to_string(),
        )))
    }

    fn put(&self, path: &str, _body: Option<HttpBody>) -> HttpRequestBuilder {
        self.record_call(reqwest::Method::PUT, path);
        HttpRequestBuilder::new(Box::new(MockRequestBuilder::new(
            reqwest::Method::PUT,
            path.to_string(),
        )))
    }

    fn patch(&self, path: &str, _body: Option<HttpBody>) -> HttpRequestBuilder {
        self.record_call(reqwest::Method::PATCH, path);
        HttpRequestBuilder::new(Box::new(MockRequestBuilder::new(
            reqwest::Method::PATCH,
            path.to_string(),
        )))
    }
}
