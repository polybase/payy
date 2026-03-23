// lint-long-file-override allow-max-lines=700
use crate::block::Block;
use crate::cache::BlockCache;
use crate::config::Config;
use crate::constants::{
    MAX_BLOCK_PRODUCTION_DELAY, MAX_BLOCK_WAIT_DELAY, MERKLE_TREE_DEPTH, MIN_BLOCK_PRODUCTION_DELAY,
};
pub use crate::errors::Error;
use crate::errors::Result;
use crate::mempool::Mempool;
use crate::network::NetworkEvent;
use crate::network_handler::network_handler;
use crate::node::load::LoadedData;
use crate::sync::SyncWorker;
use crate::types::BlockHeight;
use crate::{sync, util};
use block_store::{BlockListOrder, BlockStore, ElementHistoryIndexEntry, StoreList};
use contextful::ResultContextExt as _;
use contracts::RollupContract;
use doomslug::{Approval, ApprovalContent, ApprovalStake, ApprovalValidated, Doomslug};
use element::Element;
use futures::Stream;
use libp2p::PeerId;
use node_interface::{ElementData, RpcError};
use p2p2::Network;
use parking_lot::{Mutex, RwLock};
use primitives::hash::CryptoHash;
use primitives::pagination::CursorChoice;
use primitives::peer::{self, Address, PeerIdSigner};
use primitives::tick_worker::TickWorker;
use prover::smirk_metadata::SmirkMetadata;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::ops::RangeBounds;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error, info, instrument};
use zk_circuits::BbBackend;
use zk_primitives::UtxoProof;

pub use self::block_format::BlockFormat;
pub use self::txn_format::TxnFormat;
pub use self::txn_format::TxnMetadata;

mod block;
mod block_format;
mod load;
mod proposal;
mod snapshot;
mod tick_worker;
mod transaction;
mod txn_format;

pub type PersistentMerkleTree = smirk::storage::Persistent<MERKLE_TREE_DEPTH, SmirkMetadata>;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
// This rename is for the config parser,
// to keep it consistent with the CLI parser
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Node is a validator
    Validator,

    /// Node is a regular node
    #[default]
    Node,

    Prover,

    MockProver,
}

impl Mode {
    pub fn is_prover(&self) -> bool {
        matches!(self, Mode::Prover | Mode::MockProver)
    }
}

/// Node client
pub struct Node {
    pub shared: Arc<NodeShared>,
    sync_worker: sync::SyncWorker,
}

pub struct NodeShared {
    /// Ethereum private key
    local_peer: PeerIdSigner,

    rollup_contracts: Arc<HashMap<u64, RollupContract>>,

    /// Config
    config: Config,

    /// Doomslug consensus (currently not used)
    doomslug: Arc<Mutex<Doomslug>>,

    /// Mempool for storing pending txns
    mempool: Mempool<Element, UtxoProof, BlockHeight, Element, Arc<Block>>,

    // Block cache (unconfirmed blocks)
    pub(crate) block_cache: Arc<Mutex<BlockCache>>,

    /// Store for Solid conesnsus blocks
    block_store: Arc<BlockStore<BlockFormat>>,

    /// Network
    network: Arc<Network<NetworkEvent>>,

    /// Smirk tree containing notes
    notes_tree: Arc<RwLock<PersistentMerkleTree>>,

    /// Internal state of node
    state: Mutex<NodeSharedState>,

    /// Sync worker is responsible for requesting blocks from other nodes
    // on the network when we are out of sync
    sync_worker: sync::SyncWorkerChannel,

    // Ticker
    pub(crate) ticker: TickWorker<NodeSharedArc>,

    /// Explicitly whitelisted IP addresses
    ///
    /// Any IP address that isn't in this set will be banned (connections will be immediately
    /// rejected)
    ///
    /// If empty, whitelisting is disabled (i.e. all IPs are allowed)
    pub whitelisted_ips: HashSet<IpAddr>,

    pub bb_backend: Arc<dyn BbBackend>,
}

pub struct NodeSharedArc(Arc<NodeShared>);

#[derive(Clone)]
pub struct NodeSharedState {
    /// Instant of last commit
    last_commit: Option<Instant>,

    /// Listeners
    listeners: Vec<mpsc::UnboundedSender<Arc<Block>>>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ElementSeenInfo {
    #[expect(dead_code)]
    pub input_height: Option<BlockHeight>,
    pub output_height: BlockHeight,
    #[expect(dead_code)]
    pub input_block_hash: Option<CryptoHash>,
    pub output_block_hash: CryptoHash,
    pub spent: bool,
}

impl Node {
    pub fn new(
        local_peer: PeerIdSigner,
        rollup_contracts: Arc<HashMap<u64, RollupContract>>,
        config: Config,
        bb_backend: Arc<dyn BbBackend>,
    ) -> Result<Self> {
        info!("Mode: {:?}", config.mode);
        info!(
            address = local_peer.address().to_hex(),
            "Peer: {:?}",
            local_peer.address().to_hex()
        );

        let LoadedData {
            block_store,
            persistent_tree,
            block: initial_block,
        } = Self::load_db_and_smirk(&config)?;

        block_store
            .migrate()
            .context("run block store migrations during node startup")?;

        let block_store = Arc::new(block_store);
        let notes_tree = Arc::new(RwLock::new(persistent_tree));

        // Add the pending proposal
        let block_cache = Arc::new(Mutex::new(BlockCache::new(initial_block.clone(), 10_000)));

        let min_block_production_delay = Duration::from_millis(MIN_BLOCK_PRODUCTION_DELAY);
        let max_block_production_delay = Duration::from_secs(MAX_BLOCK_PRODUCTION_DELAY);
        let max_block_wait_delay = Duration::from_millis(MAX_BLOCK_WAIT_DELAY);

        let doomslug = Arc::new(Mutex::new(Doomslug::new(
            initial_block.content.header.height.0,
            min_block_production_delay,
            max_block_production_delay,
            max_block_production_delay / 10,
            max_block_wait_delay,
            doomslug::DoomslugThresholdMode::TwoThirds,
        )));

        let (keypair, _) = util::generate_p2p_key();
        let network = Network::new(
            &keypair,
            vec![config.p2p.laddr.clone()].into_iter(),
            config.p2p.dial.clone().into_iter(),
            config.p2p.whitelisted_ips.clone(),
        )
        .context("initialize P2P network stack with configured addresses")?;

        let (sync_worker_sender, sync_worker_receiver) = mpsc::unbounded_channel();

        let primary_chain_id = config
            .primary_chain_id()
            .ok_or(Error::MissingPrimaryChainConfig)?;
        let primary_rollup_contract = rollup_contracts.get(&primary_chain_id).cloned().ok_or(
            Error::MissingPrimaryRollupContract {
                chain_id: primary_chain_id,
            },
        )?;

        let node_shared = Arc::new(NodeShared {
            local_peer,
            rollup_contracts: Arc::clone(&rollup_contracts),
            mempool: Mempool::default(),
            block_store,
            block_cache,
            doomslug,
            notes_tree,
            network: Arc::new(network),
            config: config.clone(),
            ticker: TickWorker::new(),
            state: Mutex::new(NodeSharedState {
                last_commit: None,
                listeners: vec![],
            }),
            sync_worker: sync::SyncWorkerChannel(sync_worker_sender.clone()),
            whitelisted_ips: config.p2p.whitelisted_ips,
            bb_backend,
        });

        let sync_worker = SyncWorker::new(
            Arc::clone(&node_shared),
            Some(primary_rollup_contract),
            config.sync_chunk_size,
            config.fast_sync_threshold,
            Duration::from_millis(config.sync_timeout_ms),
            config.mode,
            sync_worker_receiver,
            sync::SyncWorkerChannel(sync_worker_sender),
        );

        Ok(Node {
            shared: node_shared,
            sync_worker,
        })
    }

    pub async fn run(self) {
        let _network_event_handler =
            network_handler(self.shared.network.clone(), self.shared.clone());

        // Dial peers
        for peer in self.shared.config.p2p.dial.iter() {
            debug!("Dialing peer {peer}...");
            match self.shared.network.dial(peer.clone()).await {
                Ok(_) => {
                    info!("Connected to peer {}", peer)
                }
                Err(err) => {
                    error!(?err, ?peer, "Failed to dial peer");
                }
            }
        }

        // Run the ticker
        self.shared
            .ticker
            .run(NodeSharedArc(Arc::clone(&self.shared)));

        // Wait for the handlers
        tokio::select! {
            res = self.sync_worker.run() => {
                if let Err(err) =  res {
                    tracing::error!(?err, "Sync worker ended");
                }
            }
        }
    }
}

impl NodeShared {
    pub(crate) fn height(&self) -> BlockHeight {
        self.block_cache.lock().height()
    }

    pub(crate) fn max_height(&self) -> BlockHeight {
        self.block_cache.lock().max_height()
    }

    pub(crate) fn is_out_of_sync(&self) -> bool {
        self.block_cache.lock().is_out_of_sync()
    }

    pub(crate) fn root_hash(&self) -> Element {
        self.notes_tree.read().tree().root_hash()
    }

    pub(crate) fn notes_tree(&self) -> &Arc<RwLock<PersistentMerkleTree>> {
        &self.notes_tree
    }

    pub(crate) fn is_validator_for_height(&self, height: BlockHeight) -> Result<bool> {
        if self.config.mode != Mode::Validator {
            return Ok(false);
        }

        let contract = self.primary_rollup_contract_required()?;

        Ok(contract
            .validators_for_height(height.0)
            .into_iter()
            .map(peer::Address::from)
            .any(|p| self.local_peer.address() == p))
    }

    pub(super) fn primary_rollup_contract(&self) -> Option<&RollupContract> {
        let chain_id = self.config.primary_chain_id()?;
        self.rollup_contracts.get(&chain_id)
    }

    pub(super) fn primary_rollup_contract_required(&self) -> Result<&RollupContract> {
        let chain_id = self
            .config
            .primary_chain_id()
            .ok_or(Error::MissingPrimaryChainConfig)?;

        self.rollup_contracts
            .get(&chain_id)
            .ok_or(Error::MissingPrimaryRollupContract { chain_id })
    }

    pub(super) fn rollup_contract_for_chain(&self, chain_id: u64) -> Option<&RollupContract> {
        self.rollup_contracts.get(&chain_id)
    }

    pub(crate) fn get_merkle_paths(&self, elements: &[Element]) -> Result<Vec<Vec<Element>>> {
        let notes_tree = self.notes_tree.read();
        let tree = notes_tree.tree();

        elements
            .iter()
            .map(|e| {
                // Check the element is in the tree
                if !tree.contains_element(e) {
                    Err::<Vec<Element>, _>(RpcError::ElementNotFound(ElementData { element: *e }))
                        .context("merkle path requested for element not present in notes tree")?;
                }

                // Return the path
                Ok(tree.path_for(*e).siblings_deepest_first().to_vec())
            })
            .collect::<Result<Vec<Vec<Element>>>>()
    }

    pub(crate) async fn send_all(&self, event: NetworkEvent) {
        self.network.send_all(event).await
    }

    pub(crate) async fn send(&self, peer: PeerId, request: NetworkEvent) {
        self.network.send(&peer, request).await
    }

    /// My peer address
    pub(crate) fn self_peer(&self) -> Address {
        self.local_peer.address()
    }

    /// Send an accept, if we're a validator
    pub async fn send_accept(&self, approval_content: ApprovalContent) -> Result<()> {
        if self.config.mode != Mode::Validator {
            return Ok(());
        }

        // If we're out of sync and syncing then we should not be sending accepts, we probably
        // don't have the proposal anyway
        if self.is_out_of_sync() {
            return Ok(());
        }

        // Create signed accept
        let approval = approval_content.to_approval(&self.local_peer);

        // Create the accept
        // TODO: initial accept (i.e. no skips) should be sent to the next leader only,
        // skip accepts should be sent to all validators
        self.send_all(NetworkEvent::Approval(approval)).await;

        Ok(())
    }

    /// Receive an accept, if we're the leader
    #[instrument(skip(self))]
    pub(crate) async fn receive_accept(&self, approval_message: &Approval) -> Result<()> {
        info!("Received approval");

        if self.config.mode != Mode::Validator {
            return Ok(());
        }

        // Check we are the block producer / leader for this accept height
        let target_height = BlockHeight(approval_message.content.target_height);
        if self.get_leader_for_block_height(target_height) != self.self_peer() {
            return Ok(());
        }

        // TODO:
        // 1. Check if the accept is from a valid validator
        // 2. Check if the accept is for the current proposal
        // 3. Check if I am the leader of the accept
        let approval: ApprovalValidated = approval_message
            .clone()
            .try_into()
            .context("convert approval message into validated doomslug approval")?;

        // Get validator stakes
        let contract = self.primary_rollup_contract_required()?;

        let stakes = contract
            .validators_for_height(approval_message.content.target_height)
            .into_iter()
            .map(|address| {
                let address = Address::from(address);
                //  &[(ApprovalStake, bool)],
                (
                    ApprovalStake {
                        validator: address,
                        stake_this_epoch: 1,
                        stake_next_epoch: 1,
                    },
                    false,
                )
            })
            .collect::<Vec<_>>();

        // Add accept to Doomslug
        self.doomslug
            .lock()
            .on_approval(Instant::now(), &approval, &stakes);

        // TODO: check if we need to do something now, i.e. propose or send accept

        Ok(())
    }

    pub fn get_leader_for_block_height(&self, height: BlockHeight) -> Address {
        // TODO: will validators be in the same order for all nodes?
        let Some(contract) = self.primary_rollup_contract() else {
            return self.self_peer();
        };

        let validators = contract.validators_for_height(height.0);
        if validators.is_empty() {
            return self.self_peer();
        }
        let leader_index = height.0 % validators.len() as u64;
        Address::from(validators[leader_index as usize])
    }

    pub(crate) async fn handle_out_of_sync(&self) -> Result<()> {
        self.sync_worker
            .out_of_sync(self.max_height())
            .context("check whether node is out of sync before scheduling recovery")?;

        Ok(())
    }

    pub fn fetch_blocks(
        &self,
        height_range: impl RangeBounds<BlockHeight> + 'static,
        order: BlockListOrder,
    ) -> impl StoreList<Item = Result<BlockFormat>> + '_ {
        self.block_store.list(height_range, order).map(|r| {
            let (_, block) = r.context("materialize block entry from store list during fetch")?;
            Ok(block)
        })
    }

    pub fn fetch_blocks_paginated<'a>(
        &'a self,
        cursor: &'a Option<CursorChoice<BlockHeight>>,
        order: BlockListOrder,
        limit: usize,
    ) -> Result<impl Iterator<Item = Result<BlockFormat>> + 'a> {
        Ok(self
            .block_store
            .list_paginated(cursor, order, limit)
            .context("paginate block store listing")?
            .map(|r| {
                let (_, block) = r.context("materialize paginated block entry")?;
                Ok(block)
            }))
    }

    pub fn fetch_blocks_non_empty(
        &self,
        height_range: impl RangeBounds<BlockHeight> + 'static,
        order: BlockListOrder,
    ) -> impl StoreList<Item = Result<BlockFormat>> + '_ {
        self.block_store
            .list_non_empty(height_range, order)
            .map(|r| {
                let (_, block) = r.context("materialize non-empty block entry from store")?;
                Ok(block)
            })
    }

    pub fn fetch_blocks_non_empty_paginated<'a>(
        &'a self,
        cursor: &'a Option<CursorChoice<BlockHeight>>,
        order: BlockListOrder,
        limit: usize,
    ) -> Result<impl Iterator<Item = Result<BlockFormat>> + 'a> {
        Ok(self
            .block_store
            .list_non_empty_paginated(cursor, order, limit)
            .context("paginate non-empty block listing")?
            .map(|r| {
                let (_, block) = r.context("materialize paginated non-empty block entry")?;
                Ok(block)
            }))
    }

    pub(crate) fn get_block(&self, height: BlockHeight) -> Result<Option<BlockFormat>> {
        Ok(self
            .block_store
            .get(height)
            .context("fetch block by height from block store")?)
    }

    pub(crate) fn get_block_by_hash(&self, hash: CryptoHash) -> Result<Option<BlockFormat>> {
        let Some(block_height) = self
            .block_store
            .get_block_height_by_hash(hash.into_inner())
            .context("resolve block height by block hash")?
        else {
            return Ok(None);
        };

        self.get_block(block_height)
    }

    /// Returns info about when an element was first seen (as an output), and whether it was later
    /// spent (seen as an input). If the element has never been seen, returns None.
    pub(crate) fn get_element_seen_info(
        &self,
        element: Element,
    ) -> Result<Option<ElementSeenInfo>> {
        let (input_hist, output_hist) = self
            .block_store
            .get_element_history(element)
            .context("fetch element history from block store")?;
        if let Some(out) = output_hist {
            let spent = input_hist.is_some();
            Ok(Some(ElementSeenInfo {
                input_height: input_hist.as_ref().map(|h| h.block_height),
                output_height: out.block_height,
                input_block_hash: input_hist.map(|h| h.block_hash),
                output_block_hash: out.block_hash,
                spent,
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn element_history_range(
        &self,
        range: impl RangeBounds<BlockHeight>,
    ) -> Vec<ElementHistoryIndexEntry> {
        self.block_store.element_history_range(range)
    }

    pub(crate) async fn commit_stream(
        &self,
        from_height: Option<BlockHeight>,
    ) -> Pin<Box<dyn Stream<Item = Result<Arc<Block>>> + Send + '_>> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.state.lock().listeners.push(tx);

        let first_commit = rx.recv().await.unwrap();
        let rx_stream = UnboundedReceiverStream::new(rx).map(Ok);
        if let Some(from_height) = from_height {
            let max_exclusive = first_commit.content.header.height;
            let blocks_before = self
                .fetch_blocks(from_height..max_exclusive, BlockListOrder::LowestToHighest)
                .map(|r| {
                    let block_format = r?;

                    Ok(Arc::new(block_format.into_block()))
                })
                .into_iterator();

            Box::pin(
                tokio_stream::iter(blocks_before)
                    .chain(tokio_stream::once(Ok(first_commit)))
                    .chain(rx_stream)
                    .filter(move |block| match block {
                        Ok(block) => block.content.header.height >= from_height,
                        Err(_) => true,
                    }),
            )
        } else {
            Box::pin(tokio_stream::once(Ok(first_commit)).chain(rx_stream))
        }
    }

    pub(crate) fn get_txn(&self, txn_hash: [u8; 32]) -> Result<Option<(UtxoProof, TxnMetadata)>> {
        let txn = self
            .block_store
            .get_txn_by_hash(txn_hash)
            .context("fetch transaction by hash from block store")?;
        Ok(txn.map(|TxnFormat::V1(txn, metadata)| (txn, metadata)))
    }

    pub(crate) fn last_commit_time(&self) -> Option<Instant> {
        self.state.lock().last_commit
    }

    pub fn estimate_block_time(height: BlockHeight, max_height: BlockHeight) -> u64 {
        chrono::Utc::now().timestamp() as u64 - (max_height.saturating_sub(height.0))
    }
}
