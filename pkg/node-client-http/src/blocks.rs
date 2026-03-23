// lint-long-file-override allow-max-lines=300
use crate::{Error, NodeClientHttp, Result};
use client_http::serde_to_query_params;
use http_interface::HttpMetadata;
use node_interface::{ListBlocksOrder, ListBlocksResponse};
use primitives::{
    block_height::BlockHeight,
    pagination::{CursorChoice, CursorChoiceAfter},
};
use reqwest::Method;
use serde::Serialize;

const BLOCKS_PATH: &str = "/blocks";
const MAX_BLOCKS_LIMIT: usize = 256;

#[derive(Serialize)]
struct BlocksQuery {
    limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
    order: ListBlocksOrder,
    skip_empty: bool,
}

impl NodeClientHttp {
    /// Fetch blocks starting at `start_height`, optionally skipping empty blocks.
    pub async fn blocks(
        &self,
        start_height: BlockHeight,
        limit: usize,
        skip_empty: bool,
    ) -> Result<ListBlocksResponse> {
        let cursor = CursorChoice::After(CursorChoiceAfter::AfterInclusive(start_height))
            .opaque()
            .serialize()
            .map_err(|err| Error::SerdeJson(err.to_string(), blocks_metadata()))?;

        let query = BlocksQuery {
            limit: limit.min(MAX_BLOCKS_LIMIT),
            cursor: Some(cursor),
            order: ListBlocksOrder::LowestToHighest,
            skip_empty,
        };

        self.http_client
            .get(BLOCKS_PATH)
            .query(serde_to_query_params(&query))
            .exec()
            .await?
            .to_value()
            .await
    }
}

fn blocks_metadata() -> HttpMetadata {
    HttpMetadata {
        method: Method::GET,
        path: BLOCKS_PATH.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_interface::{
        ClientResponse, HttpClient, HttpClientMock,
        builder::{HttpRequestBuilder, HttpRequestExecutor},
        error::HttpRequestExecError,
    };
    use primitives::pagination::{Cursor, CursorChoice, CursorChoiceAfter};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use unimock::{MockFn, Unimock, matching};

    #[derive(Default)]
    struct RequestCapture {
        query: Mutex<Option<Vec<(String, String)>>>,
    }

    impl RequestCapture {
        fn record_query(&self, query: Vec<(String, String)>) {
            *self.query.lock().unwrap() = Some(query);
        }

        fn query_map(&self) -> HashMap<String, String> {
            self.query
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect()
        }
    }

    struct TestRequestExecutor {
        state: Arc<RequestCapture>,
        response: Option<std::result::Result<ClientResponse, HttpRequestExecError>>,
    }

    impl TestRequestExecutor {
        fn new(
            state: Arc<RequestCapture>,
            response: std::result::Result<ClientResponse, HttpRequestExecError>,
        ) -> Self {
            Self {
                state,
                response: Some(response),
            }
        }
    }

    #[async_trait::async_trait]
    impl HttpRequestExecutor for TestRequestExecutor {
        fn headers(&mut self, _headers: reqwest::header::HeaderMap) {}

        fn query(&mut self, query: Vec<(String, String)>) {
            self.state.record_query(query);
        }

        fn auth(&mut self) {}

        async fn exec(
            mut self: Box<Self>,
        ) -> std::result::Result<ClientResponse, HttpRequestExecError> {
            self.response.take().expect("response already taken")
        }
    }

    fn builder_with_response(
        capture: Arc<RequestCapture>,
        response: std::result::Result<ClientResponse, HttpRequestExecError>,
    ) -> HttpRequestBuilder {
        HttpRequestBuilder::new(Box::new(TestRequestExecutor::new(capture, response)))
    }

    fn leak_get_answer<F>(
        f: F,
    ) -> &'static (
                 dyn for<'ctx, 'path> Fn(&'ctx Unimock, &'path str) -> HttpRequestBuilder
                     + Send
                     + Sync
             )
    where
        F: for<'ctx, 'path> Fn(&'ctx Unimock, &'path str) -> HttpRequestBuilder
            + Send
            + Sync
            + 'static,
    {
        Box::leak(Box::new(f))
    }

    #[tokio::test]
    async fn blocks_sets_cursor_and_skip_flag() {
        let capture = Arc::new(RequestCapture::default());
        let http: Arc<dyn HttpClient> = Arc::new(Unimock::new(
            HttpClientMock::get
                .next_call(matching!("/blocks"))
                .answers(leak_get_answer({
                    let capture = Arc::clone(&capture);
                    move |_, _path| {
                        builder_with_response(
                            Arc::clone(&capture),
                            Err(HttpRequestExecError::ServerError(
                                "server error".to_owned(),
                                blocks_metadata(),
                            )),
                        )
                    }
                })),
        ));
        let client = NodeClientHttp::with_dyn_http_client(http);

        let _ = client.blocks(BlockHeight(42), 5, true).await;

        let params = capture.query_map();
        assert_eq!(params.get("limit"), Some(&"5".to_owned()));
        assert_eq!(params.get("skip_empty"), Some(&"true".to_owned()));
        assert_eq!(params.get("order"), Some(&"LowestToHighest".to_owned()));

        let expected_cursor =
            CursorChoice::After(CursorChoiceAfter::AfterInclusive(BlockHeight(42)))
                .opaque()
                .serialize()
                .expect("cursor serialization should succeed");
        assert_eq!(params.get("cursor"), Some(&expected_cursor));
    }

    #[tokio::test]
    async fn blocks_clamps_limit_and_returns_response() {
        let capture = Arc::new(RequestCapture::default());
        let response_body = ListBlocksResponse {
            blocks: Vec::new(),
            cursor: Cursor::<BlockHeight>::default().into_opaque(),
        };

        let http: Arc<dyn HttpClient> = Arc::new(Unimock::new(
            HttpClientMock::get
                .next_call(matching!("/blocks"))
                .answers(leak_get_answer({
                    let capture = Arc::clone(&capture);
                    let response_body = response_body.clone();
                    move |_, _path| {
                        builder_with_response(
                            Arc::clone(&capture),
                            Ok(ClientResponse::from_serializable(
                                &response_body,
                                blocks_metadata(),
                            )),
                        )
                    }
                })),
        ));

        let client = NodeClientHttp::with_dyn_http_client(http);
        let result = client
            .blocks(BlockHeight(7), 300, false)
            .await
            .expect("request should succeed");
        assert!(result.blocks.is_empty());
        assert!(result.cursor.after.is_none());
        assert!(result.cursor.before.is_none());

        let params = capture.query_map();
        assert_eq!(params.get("limit"), Some(&MAX_BLOCKS_LIMIT.to_string()));
        assert_eq!(params.get("skip_empty"), Some(&"false".to_owned()));
    }
}
