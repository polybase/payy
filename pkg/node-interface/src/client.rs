#![allow(clippy::ignored_unit_patterns)]

use async_trait::async_trait;
use element::Element;
use primitives::block_height::BlockHeight;
use zk_primitives::UtxoProof;

use crate::{
    BlockTreeDiff, BlockTreeSnapshot, ElementsResponse, HeightResponse, ListBlocksResponse,
    ListTxnsParams, ListTxnsResponse, MerklePathResponse, Result, TransactionResponse,
};

/// Node client interface that supports interacting with a validator over RPC.
#[unimock::unimock(api = NodeClientMock)]
#[async_trait]
pub trait NodeClient: Send + Sync {
    /// Fetch the latest chain height and root hash.
    async fn height(&self) -> Result<HeightResponse>;

    /// Lookup elements within the commitment tree.
    async fn elements(&self, elements: &[Element], include_spent: bool)
    -> Result<ElementsResponse>;

    /// Submit a transaction to the validator.
    async fn transaction(&self, proof: UtxoProof) -> Result<TransactionResponse>;

    /// List transactions from the validator.
    async fn list_transactions(&self, params: ListTxnsParams) -> Result<ListTxnsResponse>;

    /// Fetch Merkle inclusion paths for the provided commitments.
    async fn merkle_paths(&self, commitments: &[Element]) -> Result<MerklePathResponse>;

    /// Fetch blocks from a starting height.
    async fn blocks(
        &self,
        start_height: BlockHeight,
        limit: usize,
        skip_empty: bool,
    ) -> Result<ListBlocksResponse>;

    /// Fetch a snapshot of the block tree for the provided height.
    async fn block_tree(&self, height: BlockHeight) -> Result<BlockTreeSnapshot>;

    /// Fetch a diff for the block tree from `diff_from` to the provided height.
    async fn block_tree_diff(
        &self,
        height: BlockHeight,
        diff_from: BlockHeight,
    ) -> Result<BlockTreeDiff>;
}
