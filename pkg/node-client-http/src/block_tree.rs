// lint-long-file-override allow-max-lines=300
use crate::{Error, NodeClientHttp, Result};
use http_interface::HttpMetadata;
use node_interface::{BlockTreeDiff, BlockTreeResponse, BlockTreeSnapshot};
use primitives::block_height::BlockHeight;
use reqwest::Method;

const BLOCKS_PATH: &str = "/blocks";

impl NodeClientHttp {
    /// Fetch the block tree snapshot for the provided height.
    pub async fn block_tree(&self, height: BlockHeight) -> Result<BlockTreeSnapshot> {
        let path = block_tree_path(height);
        let metadata = HttpMetadata {
            method: Method::GET,
            path: path.clone(),
        };
        self.http_client
            .get(&path)
            .exec()
            .await?
            .to_value()
            .await
            .and_then(|response: BlockTreeResponse| {
                response.elements.map_or_else(
                    || {
                        Err(Error::ServerError(
                            "missing block tree snapshot elements".to_owned(),
                            metadata,
                        ))
                    },
                    |elements| {
                        Ok(BlockTreeSnapshot {
                            height: response.height,
                            root_hash: response.root_hash,
                            elements,
                        })
                    },
                )
            })
    }

    /// Fetch the block tree diff from `diff_from` to `height`.
    pub async fn block_tree_diff(
        &self,
        height: BlockHeight,
        diff_from: BlockHeight,
    ) -> Result<BlockTreeDiff> {
        let path = block_tree_path(height);
        let metadata = HttpMetadata {
            method: Method::GET,
            path: path.clone(),
        };
        self.http_client
            .get(&path)
            .query(vec![("diff_from".to_owned(), diff_from.0.to_string())])
            .exec()
            .await?
            .to_value()
            .await
            .and_then(|response: BlockTreeResponse| {
                response.diff.map_or_else(
                    || {
                        Err(Error::ServerError(
                            "missing block tree diff response".to_owned(),
                            metadata,
                        ))
                    },
                    |diff| {
                        Ok(BlockTreeDiff {
                            height: response.height,
                            root_hash: response.root_hash,
                            diff,
                        })
                    },
                )
            })
    }
}

fn block_tree_path(height: BlockHeight) -> String {
    format!("{BLOCKS_PATH}/{}/tree", height.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use element::Element;
    use http_interface::{
        ClientResponse, HttpClient, HttpClientMock, HttpMetadata,
        builder::{HttpRequestBuilder, HttpRequestExecutor},
        error::HttpRequestExecError,
    };
    use node_interface::BlockTreeDiffChanges;
    use reqwest::Method;
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
    async fn block_tree_fetches_snapshot() {
        let height = BlockHeight(7);
        let path = block_tree_path(height);
        let expected_elements = vec![Element::new(2)];

        let http: Arc<dyn HttpClient> = Arc::new(Unimock::new(
            HttpClientMock::get
                .next_call(matching!(_))
                .answers(leak_get_answer({
                    let path = path.clone();
                    let expected_elements = expected_elements.clone();
                    move |_, actual_path| {
                        assert_eq!(actual_path, path);
                        let response_body = BlockTreeResponse {
                            height,
                            root_hash: Element::ONE,
                            elements: Some(expected_elements.clone()),
                            diff: None,
                        };
                        builder_with_response(
                            Arc::new(RequestCapture::default()),
                            Ok(ClientResponse::from_serializable(
                                &response_body,
                                HttpMetadata {
                                    method: Method::GET,
                                    path: path.clone(),
                                },
                            )),
                        )
                    }
                })),
        ));

        let client = NodeClientHttp::with_dyn_http_client(http);
        let response = client.block_tree(height).await.expect("request succeeds");
        assert_eq!(response.height, height);
        assert_eq!(response.root_hash, Element::ONE);
        assert_eq!(response.elements, expected_elements);
    }

    #[tokio::test]
    async fn block_tree_diff_sets_query() {
        let capture = Arc::new(RequestCapture::default());
        let height = BlockHeight(10);
        let diff_from = BlockHeight(5);
        let path = block_tree_path(height);
        let http: Arc<dyn HttpClient> = Arc::new(Unimock::new(
            HttpClientMock::get
                .next_call(matching!(_))
                .answers(leak_get_answer({
                    let capture = Arc::clone(&capture);
                    let path = path.clone();
                    move |_, actual_path| {
                        assert_eq!(actual_path, path);
                        builder_with_response(
                            Arc::clone(&capture),
                            Ok(ClientResponse::from_serializable(
                                &BlockTreeResponse {
                                    height,
                                    root_hash: Element::new(3),
                                    elements: None,
                                    diff: Some(BlockTreeDiffChanges {
                                        from_height: diff_from,
                                        additions: vec![Element::ONE],
                                        removals: vec![],
                                    }),
                                },
                                HttpMetadata {
                                    method: Method::GET,
                                    path: path.clone(),
                                },
                            )),
                        )
                    }
                })),
        ));

        let client = NodeClientHttp::with_dyn_http_client(http);
        let diff = client
            .block_tree_diff(height, diff_from)
            .await
            .expect("request succeeds");
        assert_eq!(diff.diff.from_height, diff_from);

        let params = capture.query_map();
        assert_eq!(params.get("diff_from"), Some(&diff_from.0.to_string()));
    }
}
