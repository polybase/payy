// lint-long-file-override allow-max-lines=300
use crate::{
    ClientHttp, ClientResponse,
    builder::RequestBuilder,
    error::{Result, handle_error_raw},
    response::HttpMetadata,
    util::wait_for_secs,
};
use http_interface::{builder::BodyApplier, error::HttpRequestExecError};
use reqwest::{Method, header::HeaderMap};
use rpc::error::{ErrorOutput, TryFromHTTPError};
use std::{fmt::Debug, sync::Arc};

pub(crate) struct RequestData {
    path: String,
    method: Method,
    headers: Option<HeaderMap>,
    body: Option<Arc<BodyApplier>>,
    query: Option<Vec<(String, String)>>,
    auth: bool,
    attempts: u64,
    auth_refreshed: bool,
}

impl From<&RequestBuilder> for RequestData {
    fn from(builder: &RequestBuilder) -> Self {
        Self {
            path: builder.path.clone(),
            method: builder.method.clone(),
            headers: builder.headers.clone(),
            body: builder.body.clone(),
            query: builder.query.clone(),
            auth: builder.auth,
            attempts: 0,
            auth_refreshed: false,
        }
    }
}

impl RequestData {
    fn register_attempt(&mut self) {
        self.attempts += 1;
    }

    fn register_refresh_auth(&mut self) {
        self.auth_refreshed = true;
        self.auth = true;
    }
}

impl ClientHttp {
    /// Send a HTTPS request producing typed errors.
    #[allow(dead_code)]
    pub(crate) async fn request<E>(&self, req_data: RequestData) -> Result<ClientResponse, E>
    where
        E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug + Send,
    {
        self.request_raw(req_data)
            .await
            .map_err(HttpRequestExecError::into_error::<E>)
    }

    /// Send a HTTPS request returning the erased execution error used by dyn dispatch.
    pub(crate) async fn request_raw(
        &self,
        mut req_data: RequestData,
    ) -> std::result::Result<ClientResponse, HttpRequestExecError> {
        loop {
            match self.execute_request(&req_data).await {
                Ok(res) => return Ok(res),
                Err(HttpRequestExecError::ServerError(message, metadata)) => {
                    if req_data.attempts < 10 {
                        wait_for_secs(1).await;
                        req_data.register_attempt();
                        continue;
                    }
                    return Err(HttpRequestExecError::ServerError(message, metadata));
                }
                Err(HttpRequestExecError::Reqwest {
                    message,
                    metadata,
                    is_connect,
                    is_timeout,
                }) => {
                    if (is_connect || is_timeout) && req_data.attempts < 10 {
                        wait_for_secs(1).await;
                        req_data.register_attempt();
                        continue;
                    }

                    return Err(HttpRequestExecError::Reqwest {
                        message,
                        metadata,
                        is_connect,
                        is_timeout,
                    });
                }
                Err(HttpRequestExecError::Unauthenticated {
                    output,
                    metadata,
                    reason,
                }) => {
                    if !req_data.auth_refreshed {
                        self.auth.refresh_auth().await.map_err(|err| {
                            HttpRequestExecError::RefreshAuth(err.to_string(), metadata.clone())
                        })?;
                        req_data.register_refresh_auth();
                        continue;
                    }
                    return Err(HttpRequestExecError::Unauthenticated {
                        output,
                        metadata,
                        reason,
                    });
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// Send a HTTPS request
    async fn execute_request(
        &self,
        req_data: &RequestData,
    ) -> std::result::Result<ClientResponse, HttpRequestExecError> {
        let url = format!("{}{}", self.base_url, req_data.path);

        let mut req_headers = self.headers.clone();

        if let Some(headers) = &req_data.headers {
            req_headers.extend(headers.clone());
        }

        let metadata = HttpMetadata {
            method: req_data.method.clone(),
            path: req_data.path.clone(),
        };

        if req_data.auth {
            let auth =
                self.auth.get_auth().await.map_err(|err| {
                    HttpRequestExecError::GetAuth(err.to_string(), metadata.clone())
                })?;
            req_headers.extend(auth);
        }

        let mut request_builder = self
            .http_client
            .request(req_data.method.clone(), &url)
            .headers(req_headers);

        if let Some(q) = &req_data.query {
            request_builder = request_builder.query(q);
        }

        if let Some(body) = &req_data.body {
            request_builder = body(request_builder);
        }

        let response =
            request_builder
                .send()
                .await
                .map_err(|err| HttpRequestExecError::Reqwest {
                    message: err.to_string(),
                    metadata: metadata.clone(),
                    is_connect: err.is_connect(),
                    is_timeout: err.is_timeout(),
                })?;

        match handle_error_raw(response, &req_data.method, &req_data.path).await {
            Ok(response) => Ok(ClientResponse::new(response, metadata)),
            Err(err) => Err(err),
        }
    }
}
