use crate::{ClientHttp, ClientResponse, error::Result, request::RequestData};
use http_interface::{
    builder::{BodyApplier, HttpRequestExecutor},
    error::HttpRequestExecError,
};
use reqwest::{Method, header::HeaderMap};
use rpc::error::{ErrorOutput, TryFromHTTPError};
use std::{fmt::Debug, sync::Arc};

/// Request configures a request to be sent.
pub struct RequestBuilder {
    pub(crate) client: ClientHttp,
    pub(crate) path: String,
    pub(crate) method: Method,
    pub(crate) headers: Option<HeaderMap>,
    pub(crate) body: Option<Arc<BodyApplier>>,
    pub(crate) query: Option<Vec<(String, String)>>,
    pub(crate) auth: bool,
}

impl RequestBuilder {
    pub fn new(
        client: ClientHttp,
        path: String,
        method: Method,
        body: Option<Arc<BodyApplier>>,
    ) -> Self {
        Self {
            client,
            path,
            method,
            headers: None,
            body,
            auth: false,
            query: None,
        }
    }

    fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set the request headers.
    pub fn headers(self, headers: HeaderMap) -> Self {
        self.with_headers(headers)
    }

    fn with_query(mut self, query: Vec<(String, String)>) -> Self {
        self.query = Some(query);
        self
    }

    /// Attach query parameters to the request.
    pub fn query(self, query: Vec<(String, String)>) -> Self {
        self.with_query(query)
    }

    fn with_auth(mut self) -> Self {
        self.auth = true;
        self
    }

    /// Enable auth header injection for the request.
    pub fn auth(self) -> Self {
        self.with_auth()
    }

    /// Execute the request while borrowing the builder to maintain the
    /// previous API surface.
    pub async fn exec<E>(&self) -> Result<ClientResponse, E>
    where
        E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug + Send,
    {
        self.exec_raw()
            .await
            .map_err(HttpRequestExecError::into_error::<E>)
    }

    pub async fn exec_owned<E>(self) -> Result<ClientResponse, E>
    where
        E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug + Send,
    {
        self.exec_owned_raw()
            .await
            .map_err(HttpRequestExecError::into_error::<E>)
    }

    pub async fn exec_raw(&self) -> std::result::Result<ClientResponse, HttpRequestExecError> {
        let client = self.client.clone();
        let request_data = RequestData::from(self);
        client.request_raw(request_data).await
    }

    pub async fn exec_owned_raw(self) -> std::result::Result<ClientResponse, HttpRequestExecError> {
        let client = self.client.clone();
        let request_data = RequestData::from(&self);
        client.request_raw(request_data).await
    }
}

#[async_trait::async_trait]
impl HttpRequestExecutor for RequestBuilder {
    fn headers(&mut self, headers: HeaderMap) {
        self.headers = Some(headers);
    }

    fn query(&mut self, query: Vec<(String, String)>) {
        self.query = Some(query);
    }

    fn auth(&mut self) {
        self.auth = true;
    }

    async fn exec(self: Box<Self>) -> std::result::Result<ClientResponse, HttpRequestExecError> {
        self.exec_owned_raw().await
    }
}
