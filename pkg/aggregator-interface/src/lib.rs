mod error;

use std::sync::Arc;

use async_trait::async_trait;
use barretenberg_interface::BbBackend;
use element::Element;
use node_interface::BlockWithInfo;
use unimock::unimock;
use zk_primitives::{AggAggProof, AggFinalProof, UtxoProofBundleWithMerkleProofs};

pub use error::{BlockProverError, ContractError, Error};

pub const UTXO_AGG_NUMBER: usize = 3;
pub const UTXO_AGGREGATIONS: usize = 2;

#[unimock(api=PrioritizableBbBackendMock)]
pub trait PrioritizableBbBackend: BbBackend + Send + Sync {
    fn with_priority(&self, priority: u64) -> Arc<dyn PrioritizableBbBackend>;
}

/// Data required to prove a batch of blocks.
#[derive(Debug, Clone)]
pub struct PreparedBatch {
    pub blocks: Vec<BlockWithInfo>,
    pub prepared_blocks: Vec<PreparedBlock>,
    pub old_root: Element,
    pub new_root: Element,
}

/// A proven batch ready for submission.
#[derive(Debug, Clone)]
pub struct ProvenBatch {
    pub final_proof: AggFinalProof,
    pub blocks: Vec<BlockWithInfo>,
    pub other_hash: [u8; 32],
}

#[unimock(api=AggregatorMock)]
#[async_trait]
pub trait Aggregator: Send + Sync {
    /// Prepares the next available batch of blocks for aggregation.
    async fn prepare_next_batch(&self) -> Result<PreparationOutcome, Error>;

    /// Generates an aggregation proof for the provided batch.
    async fn prove_batch(
        &self,
        batch: PreparedBatch,
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<ProvenBatch, Error>;

    /// Submits the proven batch to the rollup contract.
    async fn submit_batch(&self, batch: ProvenBatch) -> Result<(), Error>;
}

/// Result of a batch preparation attempt.
#[derive(Debug, Clone)]
pub enum PreparationOutcome {
    Success(PreparedBatch),
    InsufficientBlocks {
        start_height: u64,
        available: usize,
        required: usize,
    },
}

#[derive(Debug, Clone)]
pub struct RollupInput {
    pub proof: Vec<u8>,
    pub old_root: Element,
    pub new_root: Element,
    pub commit_hash: Element,
    pub utxo_messages: Vec<Element>,
    pub kzg: Vec<Element>,
    pub other_hash: [u8; 32],
    pub height: u64,
    pub signatures: Vec<Vec<u8>>,
    pub gas_per_burn_call: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedBlock {
    pub height: u64,
    pub chunks: [PreparedChunk; UTXO_AGGREGATIONS],
}

impl PreparedBlock {
    #[must_use]
    pub fn height(&self) -> u64 {
        self.height
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedChunk {
    pub old_root: Element,
    pub new_root: Element,
    pub bundles: [UtxoProofBundleWithMerkleProofs; UTXO_AGG_NUMBER],
}

#[unimock(api=RollupContractMock)]
#[async_trait]
pub trait RollupContract: Send + Sync {
    async fn height(&self) -> Result<u64, ContractError>;
    async fn root_hash(&self) -> Result<Element, ContractError>;
    async fn submit_rollup(&self, rollup: &RollupInput) -> Result<(), ContractError>;
}

#[unimock(api=BlockProverMock)]
#[async_trait]
pub trait BlockProver: Send + Sync {
    async fn prepare(
        &self,
        height: u64,
        tree: &mut dyn RollupTree,
    ) -> Result<PreparedBlock, BlockProverError>;

    async fn prove(
        &self,
        prepared: PreparedBlock,
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<AggAggProof, BlockProverError>;
}

#[unimock(api=RollupTreeMock)]
pub trait RollupTree: Send {
    fn root_hash(&self) -> Element;
    fn height(&self) -> u64;
    fn set_height(&mut self, height: u64);
    fn sibling_path(&self, element: Element) -> Result<Vec<Element>, BlockProverError>;
    fn remove(&mut self, element: Element) -> Result<(), BlockProverError>;
    fn insert(&mut self, entries: &[(Element, u64)]) -> Result<(), BlockProverError>;
}
