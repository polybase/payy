use aggregator_interface::BlockProverError;
use contextful::Contextful;
use element::Element;
use node_interface::Error as NodeClientError;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Error)]
pub enum BlockProverImplError {
    #[error("[aggregator/block_prover] node client error: {0}")]
    Node(#[from] Contextful<NodeClientError>),
    #[error("[aggregator/block_prover] missing block for height {0}")]
    MissingBlock(u64),
    #[error("[aggregator/block_prover] block height mismatch: expected {expected}, got {found}")]
    BlockHeightMismatch { expected: u64, found: u64 },
    #[error("[aggregator/block_prover] diff height mismatch: expected {expected}, got {found}")]
    DiffHeightMismatch { expected: u64, found: u64 },
    #[error(
        "[aggregator/block_prover] diff from height mismatch: expected {expected}, got {found}"
    )]
    DiffFromMismatch { expected: u64, found: u64 },
    #[error("[aggregator/block_prover] too many transactions: max {max}, got {found}")]
    TooManyTransactions { found: usize, max: usize },
    #[error("[aggregator/block_prover] chunk count mismatch: expected {expected}, got {found}")]
    ChunkCountMismatch { expected: usize, found: usize },
    #[error("[aggregator/block_prover] bundle count mismatch: expected {expected}, got {found}")]
    BundleCountMismatch { expected: usize, found: usize },
    #[error(
        "[aggregator/block_prover] merkle path length mismatch: expected {expected}, got {found}"
    )]
    MerklePathLength { expected: usize, found: usize },
    #[error("[aggregator/block_prover] root mismatch: expected {expected:?}, got {got:?}")]
    RootMismatch { expected: Element, got: Element },
    #[error("[aggregator/block_prover] circuit error: {0}")]
    Circuit(#[from] Contextful<zk_circuits::Error>),
    #[error("[aggregator/block_prover] join error: {0}")]
    Join(#[from] Contextful<JoinError>),
}

impl From<BlockProverImplError> for BlockProverError {
    fn from(value: BlockProverImplError) -> Self {
        BlockProverError::ImplementationSpecific(Box::new(value))
    }
}
