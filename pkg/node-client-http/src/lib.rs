#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! HTTPS client for Payy Network

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use client_http::{ClientHttp, NoAuth};
use element::Element;
use http_interface::HttpClient;
use node_interface::{
    BlockTreeDiff, BlockTreeSnapshot, ElementsResponse, Error as NodeInterfaceError,
    HeightResponse, ListBlocksResponse, MerklePathResponse, NodeClient,
    Result as NodeInterfaceResult, TransactionResponse,
};
use primitives::block_height::BlockHeight;
use zk_primitives::UtxoProof;

pub use reqwest::{Url, header::HeaderMap};

mod block_tree;
mod blocks;
mod elements;
mod error;
mod height;
mod merkle;
mod transaction;

pub use error::{Error, Result};

/// Node client for interfacting with the Payy Network validator
/// over HTTPS
#[derive(Clone)]
pub struct NodeClientHttp {
    pub(crate) http_client: Arc<dyn HttpClient>,
}

impl NodeClientHttp {
    /// Create a new client with a base URL using the default HTTP implementation.
    #[must_use]
    pub fn new(base_url: Url) -> Self {
        Self::with_http_client(ClientHttp::new(base_url, HeaderMap::default(), NoAuth))
    }

    /// Construct a node client with a pre-configured HTTP implementation.
    #[must_use]
    pub fn with_http_client<C>(http_client: C) -> Self
    where
        C: HttpClient + 'static,
    {
        Self::with_dyn_http_client(Arc::new(http_client))
    }

    /// Construct a node client backed by a trait-object HTTP client.
    #[must_use]
    pub fn with_dyn_http_client(http_client: Arc<dyn HttpClient>) -> Self {
        Self { http_client }
    }
}

#[async_trait]
impl NodeClient for NodeClientHttp {
    async fn height(&self) -> NodeInterfaceResult<HeightResponse> {
        NodeClientHttp::height(self).await.map_err(map_client_error)
    }

    async fn elements(
        &self,
        elements: &[Element],
        include_spent: bool,
    ) -> NodeInterfaceResult<ElementsResponse> {
        NodeClientHttp::elements(self, elements, include_spent)
            .await
            .map_err(map_client_error)
    }

    async fn transaction(&self, proof: UtxoProof) -> NodeInterfaceResult<TransactionResponse> {
        NodeClientHttp::transaction(self, proof)
            .await
            .map_err(map_client_error)
    }

    async fn list_transactions(
        &self,
        params: node_interface::ListTxnsParams,
    ) -> NodeInterfaceResult<node_interface::ListTxnsResponse> {
        NodeClientHttp::list_transactions(self, params)
            .await
            .map_err(map_client_error)
    }

    async fn merkle_paths(
        &self,
        commitments: &[Element],
    ) -> NodeInterfaceResult<MerklePathResponse> {
        NodeClientHttp::merkle_paths(self, commitments)
            .await
            .map_err(map_client_error)
    }

    async fn blocks(
        &self,
        start_height: BlockHeight,
        limit: usize,
        skip_empty: bool,
    ) -> NodeInterfaceResult<ListBlocksResponse> {
        NodeClientHttp::blocks(self, start_height, limit, skip_empty)
            .await
            .map_err(map_client_error)
    }

    async fn block_tree(&self, height: BlockHeight) -> NodeInterfaceResult<BlockTreeSnapshot> {
        NodeClientHttp::block_tree(self, height)
            .await
            .map_err(map_client_error)
    }

    async fn block_tree_diff(
        &self,
        height: BlockHeight,
        diff_from: BlockHeight,
    ) -> NodeInterfaceResult<BlockTreeDiff> {
        NodeClientHttp::block_tree_diff(self, height, diff_from)
            .await
            .map_err(map_client_error)
    }
}

fn map_client_error(err: Error) -> NodeInterfaceError {
    match err {
        client_http::Error::Rpc(rpc_error) => rpc_error.into(),
        other => NodeInterfaceError::from(Box::new(other) as Box<dyn StdError + Send + Sync>),
    }
}
