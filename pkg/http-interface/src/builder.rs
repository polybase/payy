use crate::{
    ClientResponse,
    error::{HttpRequestExecError, Result},
};
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use rpc::error::{ErrorOutput, TryFromHTTPError};
use serde::Serialize;
use std::{fmt::Debug, sync::Arc};

/// Shared function signature applied to a [`reqwest::RequestBuilder`] before dispatch.
pub type BodyApplier =
    dyn Fn(reqwest::RequestBuilder) -> reqwest::RequestBuilder + Send + Sync + 'static;

/// Owned request body that can be replayed when the request executes.
pub struct HttpBody {
    applier: Arc<BodyApplier>,
}

impl HttpBody {
    /// Construct a JSON body from an owned serializable value.
    pub fn json<T>(value: T) -> Self
    where
        T: Serialize + Send + Sync + 'static,
    {
        let value = Arc::new(value);
        Self {
            applier: Arc::new(move |builder: reqwest::RequestBuilder| builder.json(&*value)),
        }
    }

    /// Access the request mutation closure used when executing the HTTP call.
    #[must_use]
    pub fn applier(&self) -> Arc<BodyApplier> {
        Arc::clone(&self.applier)
    }
}

/// Internal adapter trait that bridges concrete request builders to the type-erased interface.
#[async_trait]
pub trait HttpRequestExecutor: Send {
    /// Attach additional headers to the request being built.
    fn headers(&mut self, headers: HeaderMap);

    /// Attach query parameters to the request being built.
    fn query(&mut self, query: Vec<(String, String)>);

    /// Request that authentication be applied to the built request.
    fn auth(&mut self);

    /// Execute the request, yielding the raw client response or execution error.
    async fn exec(self: Box<Self>) -> std::result::Result<ClientResponse, HttpRequestExecError>;
}

/// Type-erased HTTP request builder returned by [`crate::HttpClient`].
pub struct HttpRequestBuilder {
    executor: Box<dyn HttpRequestExecutor>,
}

impl HttpRequestBuilder {
    /// Create a new builder from the provided executor.
    #[must_use]
    pub fn new(executor: Box<dyn HttpRequestExecutor>) -> Self {
        Self { executor }
    }

    /// Attach additional headers to the request.
    #[must_use]
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.executor.headers(headers);
        self
    }

    /// Attach query parameters to the request.
    #[must_use]
    pub fn query(mut self, query: Vec<(String, String)>) -> Self {
        self.executor.query(query);
        self
    }

    /// Request that the underlying client applies authentication before sending.
    #[must_use]
    pub fn auth(mut self) -> Self {
        self.executor.auth();
        self
    }

    /// Execute the HTTP request and return the wrapped response.
    pub async fn exec<E>(self) -> Result<ClientResponse, E>
    where
        E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug + Send,
    {
        self.executor
            .exec()
            .await
            .map_err(HttpRequestExecError::into_error::<E>)
    }
}
