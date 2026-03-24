// lint-long-file-override allow-max-lines=400
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use aggregator::{
    AggAggCircuitInterface, AggFinalCircuitInterface, Aggregator, ContractsRollupContract,
};
use aggregator_interface::{
    Aggregator as AggregatorTrait, BlockProver as BlockProverTrait, BlockProverError,
    PreparationOutcome, PreparedBlock, PreparedChunk, RollupTree, UTXO_AGG_NUMBER,
    UTXO_AGGREGATIONS,
};
use barretenberg_interface::{BbBackend, error::Error as BbError};
use element::Element;
use node_client_http::{Error as NodeClientHttpError, NodeClientHttp};
use node_interface::{Error as NodeInterfaceError, NodeClient};
use primitives::block_height::BlockHeight;
use rpc::code::ErrorCode;
use testutil::eth::EthNode;
use zk_circuits::{AggAggCircuit, AggFinalCircuit};
use zk_primitives::{
    AggAggProof, AggAggPublicInput, AggFinalPublicInput, ProofBytes,
    UtxoProofBundleWithMerkleProofs,
};

use super::{Server, rollup_contract};

const BYTES_PER_ELEMENT: usize = 32;
const AGG_AGG_PUBLIC_INPUTS_COUNT: usize = 2 + 1 + 1 + 1 + 1000;
const AGG_AGG_PROOF_ELEMENT_COUNT: usize = 508;
const AGG_FINAL_PUBLIC_INPUTS_COUNT: usize = 1 + 1 + 1 + 1000;
const AGG_FINAL_PROOF_ELEMENT_COUNT: usize = 330;
const MERKLE_TREE_DEPTH: usize = 161;
const MERKLE_TREE_PATH_DEPTH: usize = MERKLE_TREE_DEPTH - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMockAggregatorOutcome {
    NoPendingBatch,
    SubmittedBatch,
}

pub async fn run_mock_aggregator(
    server: &Server,
    eth_node: &Arc<EthNode>,
) -> RunMockAggregatorOutcome {
    let rollup: Arc<contracts::RollupContract> =
        Arc::new(rollup_contract(server.rollup_contract_addr, eth_node).await);
    let node_client: Arc<dyn NodeClient> = Arc::new(NodeClientHttp::new(server.base_url()));
    let block_prover = Arc::new(MockBlockProver::new(Arc::clone(&node_client)));
    let backend = Arc::new(MockBbBackend::default());

    let (rolled_root, rolled_height) = plan_mock_rollup(rollup.as_ref(), &backend).await;

    let rollup_tree: Box<dyn RollupTree> =
        Box::new(MockRollupTree::new(rolled_root, rolled_height));
    let contract_adapter = Arc::new(ContractsRollupContract::new(
        Arc::clone(&rollup),
        std::time::Duration::from_secs(60),
        std::time::Duration::from_millis(100),
    ));

    let node_height = server.height().await.unwrap();
    assert!(
        node_height.height.0 >= rolled_height,
        "mock aggregator observed node height below rollup height; node={}, rollup={rolled_height}",
        node_height.height.0,
    );
    if node_height.height.0 == rolled_height
        || !has_pending_non_empty_blocks(node_client.as_ref(), rolled_height).await
    {
        return RunMockAggregatorOutcome::NoPendingBatch;
    }

    let agg_agg_circuit: Arc<dyn AggAggCircuitInterface> = Arc::new(AggAggCircuit);
    let agg_final_circuit: Arc<dyn AggFinalCircuitInterface> = Arc::new(AggFinalCircuit);
    let aggregator = Aggregator::new(
        node_client,
        contract_adapter,
        block_prover,
        rollup_tree,
        2,
        1_000_000,
        agg_agg_circuit,
        agg_final_circuit,
    );

    let batch = match aggregator
        .prepare_next_batch()
        .await
        .expect("prepare next aggregator batch")
    {
        PreparationOutcome::Success(batch) => batch,
        PreparationOutcome::InsufficientBlocks { .. } => {
            return RunMockAggregatorOutcome::NoPendingBatch;
        }
    };
    let proven = aggregator
        .prove_batch(batch, Arc::clone(&backend) as Arc<dyn BbBackend>)
        .await
        .expect("prove aggregator batch");
    aggregator
        .submit_batch(proven)
        .await
        .expect("submit aggregator batch");

    let contract_height = rollup.block_height().await.unwrap();
    let contract_root = Element::from_be_bytes(rollup.root_hash().await.unwrap().to_fixed_bytes());
    let node_height = server.height().await.unwrap();
    assert_eq!(contract_height, node_height.height.0);
    assert_eq!(contract_root, node_height.root_hash);
    RunMockAggregatorOutcome::SubmittedBatch
}

async fn plan_mock_rollup(
    rollup: &contracts::RollupContract,
    backend: &MockBbBackend,
) -> (Element, u64) {
    let rolled_height = rollup.block_height().await.unwrap();
    let rolled_root = Element::from_be_bytes(rollup.root_hash().await.unwrap().to_fixed_bytes());

    let agg_inputs = AggAggPublicInput {
        verification_key_hash: [Element::ZERO; 2],
        old_root: rolled_root,
        new_root: rolled_root,
        commit_hash: Element::ZERO,
        messages: [Element::ZERO; 1000],
    };
    backend.push_response(agg_agg_bb_output(&agg_inputs));

    let final_inputs = AggFinalPublicInput {
        old_root: rolled_root,
        new_root: rolled_root,
        commit_hash: Element::ZERO,
        messages: agg_inputs.messages.to_vec(),
    };
    backend.push_response(agg_final_bb_output(&final_inputs));

    (rolled_root, rolled_height)
}

struct MockBbBackend {
    responses: Mutex<VecDeque<Vec<u8>>>,
}

impl Default for MockBbBackend {
    fn default() -> Self {
        Self {
            responses: Mutex::new(VecDeque::new()),
        }
    }
}

impl MockBbBackend {
    fn push_response(&self, bytes: Vec<u8>) {
        self.responses.lock().unwrap().push_back(bytes);
    }
}

#[async_trait::async_trait]
impl BbBackend for MockBbBackend {
    async fn prove(
        &self,
        _program: &[u8],
        _bytecode: &[u8],
        _key: &[u8],
        _witness: &[u8],
        _oracle: bool,
    ) -> Result<Vec<u8>, BbError> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| BbError::Backend("missing mock barretenberg response".into()))
    }

    async fn verify(
        &self,
        _proof: &[u8],
        _public_inputs: &[u8],
        _key: &[u8],
        _oracle: bool,
    ) -> Result<(), BbError> {
        Ok(())
    }
}

struct MockBlockProver {
    node_client: Arc<dyn NodeClient>,
}

impl MockBlockProver {
    fn new(node_client: Arc<dyn NodeClient>) -> Self {
        Self { node_client }
    }
}

#[async_trait::async_trait]
impl BlockProverTrait for MockBlockProver {
    async fn prepare(
        &self,
        height: u64,
        tree: &mut dyn RollupTree,
    ) -> Result<PreparedBlock, BlockProverError> {
        let response = self
            .node_client
            .blocks(BlockHeight(height), 1, true)
            .await
            .map_err(map_node_error)?;
        let block = response.blocks.first().cloned().ok_or_else(|| {
            BlockProverError::ImplementationSpecific(Box::new(std::io::Error::other(format!(
                "missing block {height}"
            ))))
        })?;

        let old_root = tree.root_hash();
        let new_root = block.block.content.state.root_hash;
        tree.insert(&[(new_root, height)])?;
        tree.set_height(height);

        let mut chunks: [PreparedChunk; UTXO_AGGREGATIONS] =
            std::array::from_fn(|_| padding_chunk());
        chunks[0] = PreparedChunk {
            old_root,
            new_root,
            bundles: default_bundles(),
        };

        Ok(PreparedBlock { height, chunks })
    }

    async fn prove(
        &self,
        prepared: PreparedBlock,
        _bb_backend: Arc<dyn BbBackend>,
    ) -> Result<AggAggProof, BlockProverError> {
        let chunk = prepared.chunks[0].clone();
        Ok(AggAggProof {
            proof: ProofBytes::default(),
            public_inputs: AggAggPublicInput {
                verification_key_hash: [Element::ZERO; 2],
                old_root: chunk.old_root,
                new_root: chunk.new_root,
                commit_hash: Element::ZERO,
                messages: [Element::ZERO; 1000],
            },
            kzg: vec![],
        })
    }
}

fn default_bundles() -> [UtxoProofBundleWithMerkleProofs; UTXO_AGG_NUMBER] {
    std::array::from_fn(|_| UtxoProofBundleWithMerkleProofs::default())
}

fn padding_chunk() -> PreparedChunk {
    PreparedChunk {
        old_root: Element::ZERO,
        new_root: Element::ZERO,
        bundles: default_bundles(),
    }
}

struct MockRollupTree {
    root: Element,
    height: u64,
}

impl MockRollupTree {
    fn new(root: Element, height: u64) -> Self {
        Self { root, height }
    }
}

impl RollupTree for MockRollupTree {
    fn root_hash(&self) -> Element {
        self.root
    }

    fn height(&self) -> u64 {
        self.height
    }

    fn set_height(&mut self, height: u64) {
        self.height = height;
    }

    fn sibling_path(&self, _element: Element) -> Result<Vec<Element>, BlockProverError> {
        Ok(vec![Element::ZERO; MERKLE_TREE_PATH_DEPTH])
    }

    fn remove(&mut self, _element: Element) -> Result<(), BlockProverError> {
        Ok(())
    }

    fn insert(&mut self, entries: &[(Element, u64)]) -> Result<(), BlockProverError> {
        if let Some((element, _)) = entries.last() {
            self.root = *element;
        }
        Ok(())
    }
}

fn agg_agg_bb_output(inputs: &AggAggPublicInput) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        AGG_AGG_PUBLIC_INPUTS_COUNT * BYTES_PER_ELEMENT
            + AGG_AGG_PROOF_ELEMENT_COUNT * BYTES_PER_ELEMENT,
    );

    for element in &inputs.verification_key_hash {
        bytes.extend_from_slice(&element.to_be_bytes());
    }
    bytes.extend_from_slice(&inputs.old_root.to_be_bytes());
    bytes.extend_from_slice(&inputs.new_root.to_be_bytes());
    bytes.extend_from_slice(&inputs.commit_hash.to_be_bytes());

    for message in &inputs.messages {
        bytes.extend_from_slice(&message.to_be_bytes());
    }

    bytes.extend(vec![4u8; AGG_AGG_PROOF_ELEMENT_COUNT * BYTES_PER_ELEMENT]);
    bytes
}

fn agg_final_bb_output(inputs: &AggFinalPublicInput) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        AGG_FINAL_PUBLIC_INPUTS_COUNT * BYTES_PER_ELEMENT
            + AGG_FINAL_PROOF_ELEMENT_COUNT * BYTES_PER_ELEMENT,
    );

    bytes.extend_from_slice(&inputs.old_root.to_be_bytes());
    bytes.extend_from_slice(&inputs.new_root.to_be_bytes());
    bytes.extend_from_slice(&inputs.commit_hash.to_be_bytes());

    for message in &inputs.messages {
        bytes.extend_from_slice(&message.to_be_bytes());
    }

    bytes.extend(vec![5u8; AGG_FINAL_PROOF_ELEMENT_COUNT * BYTES_PER_ELEMENT]);
    bytes
}

fn map_node_error(err: NodeInterfaceError) -> BlockProverError {
    BlockProverError::ImplementationSpecific(Box::new(err))
}

async fn has_pending_non_empty_blocks(node_client: &dyn NodeClient, rolled_height: u64) -> bool {
    let start_height = rolled_height
        .checked_add(1)
        .expect("rolled height overflow while checking pending aggregator blocks");
    match node_client.blocks(BlockHeight(start_height), 1, true).await {
        Ok(response) => response
            .blocks
            .into_iter()
            .any(|block| block.block.content.header.height.0 >= start_height),
        Err(err) if is_blocks_not_found(&err) => false,
        Err(err) => panic!("query pending aggregator blocks: {err}"),
    }
}

fn is_blocks_not_found(err: &NodeInterfaceError) -> bool {
    let NodeInterfaceError::Client(err) = err else {
        return false;
    };
    let Some(err) = err.as_ref().downcast_ref::<NodeClientHttpError>() else {
        return false;
    };

    matches!(
        err,
        NodeClientHttpError::UnknownErrorOutput(output, _, metadata)
            if output.error.code == ErrorCode::NotFound && metadata.path == "/blocks"
    )
}
