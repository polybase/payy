//! This module contains the sync worker, which is responsible for
//! synchronizing the node with the rest of the network.
//! It is responsible for requesting and applying snapshots.
//! It expects to be sent snapshot network messages (as [Message])
//! via the [SyncWorkerChannel].
//! The main entry point is [SyncWorker::run].

// lint-long-file-override allow-max-lines=700
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use block_store::{BlockListOrder, StoreList};
use contracts::RollupContract;
use element::Element;
use libp2p::PeerId;
use parking_lot::Mutex;
use prover::smirk_metadata::SmirkMetadata;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::{
    Error as NodeError, NodeShared, PersistentMerkleTree,
    block::Block,
    cache::BlockCache,
    network::{
        NetworkEvent, SnapshotAccept, SnapshotChunk, SnapshotChunkFast, SnapshotChunkSlow,
        SnapshotKind, SnapshotOffer as NetworkSnapshotOffer, SnapshotRequest,
    },
    node::Mode,
    types::{BlockHeight, SnapshotId},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("channel was closed")]
    ChannelWasClosed,

    #[error("{0} send error")]
    Send(&'static str),

    #[error("node error: {0}")]
    Node(#[from] Box<NodeError>),

    #[error("smirk collision error: {0}")]
    SmirkCollision(#[from] smirk::CollisionError),

    #[error("smirk storage error: {0}")]
    SmirkStorage(#[from] smirk::storage::Error),

    #[error("tokio join error: {0}")]
    TokioJoin(#[from] tokio::task::JoinError),
}

pub enum Message {
    OutOfSync(OutOfSync),
    SnapshotOffer(SyncSnapshotOffer),
    SnapshotChunk(PeerId, SnapshotChunk),
}

/// A message signaling that the node needs to start syncing,
/// because it's behind the rest of the network.
pub struct OutOfSync {
    pub max_seen_height: BlockHeight,
}

/// Same as [crate::network::NetworkEvent::SnapshotOffer]
pub struct SyncSnapshotOffer {
    pub peer: PeerId,
    pub snapshot_id: SnapshotId,
}

/// A channel for sending messages to the sync worker.
#[derive(Clone)]
pub struct SyncWorkerChannel(pub mpsc::UnboundedSender<Message>);

impl SyncWorkerChannel {
    /// Handled by [SyncWorker::handle_out_of_sync].
    /// This is the first message the sync worker expects,
    /// to trigger the sync process.
    pub fn out_of_sync(&self, max_seen_height: BlockHeight) -> Result<(), Error> {
        self.0
            .send(Message::OutOfSync(OutOfSync { max_seen_height }))
            .map_err(|_| Error::ChannelWasClosed)
    }

    /// Handled by [SyncWorker::handle_snapshot_offer].
    pub fn snapshot_offer(&self, peer: PeerId, snapshot_id: SnapshotId) -> Result<(), Error> {
        self.0
            .send(Message::SnapshotOffer(SyncSnapshotOffer {
                peer,
                snapshot_id,
            }))
            .map_err(|_| Error::ChannelWasClosed)
    }

    /// Handled by [SyncWorker::handle_snapshot_chunk].
    pub fn snapshot_chunk(&self, peer: PeerId, sc: SnapshotChunk) -> Result<(), Error> {
        self.0
            .send(Message::SnapshotChunk(peer, sc))
            .map_err(|_| Error::ChannelWasClosed)
    }
}

pub struct SyncWorker {
    node: Arc<NodeShared>,
    rollup_contract: Option<RollupContract>,
    /// The number of blocks to request in a snapshot chunk.
    chunk_size: u64,
    /// Only request fast sync if the node is this many blocks behind.
    fast_sync_threshold: u64,
    /// Duration after which we stop waiting for a snapshot offer/chunk.
    timeout: std::time::Duration,
    node_mode: Mode,
    /// A channel for receiving sync network messages.
    channel: mpsc::UnboundedReceiver<Message>,
    /// We need the channel sender to send messages to ourselves,
    /// such that once we receive a snapshot chunk,
    /// we can trigger out of sync again,
    /// without recursing.
    channel_sender: SyncWorkerChannel,
}

impl SyncWorker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node: Arc<NodeShared>,
        rollup_contract: Option<RollupContract>,
        chunk_size: u64,
        fast_sync_threshold: u64,
        timeout: std::time::Duration,
        node_mode: Mode,
        channel: mpsc::UnboundedReceiver<Message>,
        channel_sender: SyncWorkerChannel,
    ) -> Self {
        Self {
            node,
            rollup_contract,
            chunk_size,
            fast_sync_threshold,
            timeout,
            node_mode,
            channel,
            channel_sender,
        }
    }

    pub async fn run(mut self) -> Result<(), Error> {
        tokio::spawn(async move {
            loop {
                let out_of_sync = self.wait_for_out_of_sync().await?;
                self.handle_out_of_sync(out_of_sync).await?;

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        })
        .await?
    }

    pub async fn wait_for_out_of_sync(&mut self) -> Result<OutOfSync, Error> {
        while let Some(msg) = self.channel.recv().await {
            if let Message::OutOfSync(out_of_sync) = msg {
                return Ok(out_of_sync);
            }
        }

        Err(Error::ChannelWasClosed)
    }

    async fn handle_out_of_sync(
        &mut self,
        OutOfSync { max_seen_height }: OutOfSync,
    ) -> Result<(), Error> {
        if !self.node.is_out_of_sync() {
            return Ok(());
        }

        let snapshot_id = rand::random();

        let from_height = self.node.height() + BlockHeight(1);
        let mut to_height = from_height + BlockHeight(self.chunk_size);
        let mut snapshot_kind = SnapshotKind::Slow;

        let far_enough_to_try_fast_sync =
            max_seen_height.0 - self.node.height().0 > self.fast_sync_threshold;
        if self.node_mode.is_prover()
            && far_enough_to_try_fast_sync
            && let Some(rollup_contract) = &self.rollup_contract
        {
            match rollup_contract.block_height().await {
                Ok(contract_height) => {
                    let contract_height = BlockHeight(contract_height);
                    if contract_height > to_height {
                        to_height = contract_height;
                        snapshot_kind = SnapshotKind::Fast;
                    }
                }
                Err(err) => {
                    error!(?err, "Failed to get contract height");
                }
            }
        };

        info!(
            ?snapshot_id,
            node_height = ?self.node.height(),
            max_max_seen_height = ?self.node.max_height(),
            ?from_height,
            ?to_height,
            ?snapshot_kind,
            "Requesting snapshot"
        );

        let snapshot_request = SnapshotRequest {
            snapshot_id,
            from_height,
            to_height,
            kind: snapshot_kind,
        };
        let request = snapshot_request;
        self.node
            .send_all(NetworkEvent::SnapshotRequest(request))
            .await;

        let so = tokio::select! {
            so = self.wait_for_snapshot_offer(snapshot_id) => so?,
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
                warn!(?snapshot_id, "snapshot offer timed out");
                return Ok(());
            },
        };

        self.handle_snapshot_offer(snapshot_kind, to_height, so)
            .await?;

        Ok(())
    }

    async fn wait_for_snapshot_offer(
        &mut self,
        snapshot_id: SnapshotId,
    ) -> Result<SyncSnapshotOffer, Error> {
        while let Some(msg) = self.channel.recv().await {
            match msg {
                Message::SnapshotOffer(so) if so.snapshot_id == snapshot_id => return Ok(so),
                _ => {}
            }
        }

        Err(Error::ChannelWasClosed)
    }

    async fn handle_snapshot_offer(
        &mut self,
        kind: SnapshotKind,
        to_height: BlockHeight,
        SyncSnapshotOffer { peer, snapshot_id }: SyncSnapshotOffer,
    ) -> Result<(), Error> {
        let from_height = self.node.height() + BlockHeight(1);

        info!(
            ?snapshot_id,
            ?from_height,
            ?to_height,
            ?peer,
            "Accepting snapshot offer"
        );
        let accept = SnapshotAccept {
            snapshot_id,
            from_height,
            to_height,
            kind,
        };
        self.node
            .send(peer, NetworkEvent::SnapshotAccept(accept))
            .await;

        let (peer, sc) = tokio::select! {
            _ = tokio::time::sleep(self.timeout) => {
                warn!(?snapshot_id, ?peer, "snapshot chunk timed out");
                return Ok(());
            }
            sc = self.wait_for_snapshot_chunk(peer, snapshot_id) => sc?,
        };

        self.handle_snapshot_chunk(peer, sc).await?;

        Ok(())
    }

    async fn wait_for_snapshot_chunk(
        &mut self,
        peer: PeerId,
        snapshot_id: SnapshotId,
    ) -> Result<(PeerId, SnapshotChunk), Error> {
        while let Some(msg) = self.channel.recv().await {
            match msg {
                Message::SnapshotChunk(sc_peer, sc)
                    if sc_peer == peer && sc.snapshot_id() == snapshot_id =>
                {
                    return Ok((peer, sc));
                }
                _ => {}
            }
        }

        Err(Error::ChannelWasClosed)
    }

    async fn handle_snapshot_chunk(
        &mut self,
        peer: PeerId,
        sc: SnapshotChunk,
    ) -> Result<(), Error> {
        match sc {
            SnapshotChunk::Slow(sc) => self.handle_snapshot_chunk_slow(peer, sc).await,
            SnapshotChunk::Fast(sc) => self.handle_snapshot_chunk_fast(peer, sc).await,
        }
    }

    async fn handle_snapshot_chunk_slow(
        &mut self,
        peer: PeerId,
        SnapshotChunkSlow {
            snapshot_id,
            mut chunk,
        }: SnapshotChunkSlow,
    ) -> Result<(), Error> {
        let proposal_len = chunk.len();

        // Chunk can be very large, so we work on it in a blocking task
        tokio::task::spawn_blocking({
            let node = Arc::clone(&self.node);

            move || {
                chunk.sort_by_key(|b| b.content.header.height);

                for block in chunk {
                    let hash = block.hash();
                    let height = block.content.header.height;
                    match node.receive_proposal(block) {
                        Ok(_) => {}
                        Err(err) => {
                            warn!(?err, ?hash, ?height, "Failed to receive proposal");
                        }
                    }
                    node.ticker.tick()
                }
            }
        })
        .await?;

        info!(
            ?snapshot_id,
            from = ?peer,
            proposal_len,
            "Applied snapshot proposals"
        );

        if !self.node.is_out_of_sync() {
            info!(?snapshot_id, "Finished synchronizing proposals");
            return Ok(());
        }

        // Send a message rather than call [[Self::handle_out_of_sync]], so that we don't recurse
        self.channel_sender.out_of_sync(self.node.max_height())?;

        Ok(())
    }

    async fn handle_snapshot_chunk_fast(
        &mut self,
        _peer: PeerId,
        SnapshotChunkFast {
            snapshot_id: _,
            block,
            elements,
        }: SnapshotChunkFast,
    ) -> Result<(), Error> {
        let Some(block) = block else {
            warn!("Fast snapshot chunk missing block");
            return Ok(());
        };

        {
            let mut tree = self.node.notes_tree().write();
            Self::apply_fast_snapshot_chunk(&mut tree, &block, &elements)?;
            self.node
                .block_cache
                .lock()
                .confirm(block.content.header.height - BlockHeight(1))
        }

        self.node.receive_proposal(*block).map_err(Box::new)?;
        self.node.ticker.tick();

        Ok(())
    }

    fn apply_fast_snapshot_chunk(
        tree: &mut PersistentMerkleTree,
        block: &Block,
        elements: &[Element],
    ) -> Result<(), Error> {
        // Elements for the last block in the chunk
        let mut last_block_elements = HashMap::<
            Element,
            // true = add
            // false = remove
            bool,
        >::new();
        for utxo in block.content.state.txns.iter() {
            for e in &utxo.public_inputs.input_commitments {
                last_block_elements.insert(*e, false);
            }
            for e in &utxo.public_inputs.output_commitments {
                last_block_elements.insert(*e, true);
            }
        }

        // Build sets for diffing
        let elements_set: HashSet<_> = elements.iter().copied().collect();
        let tree_elements_set: HashSet<_> = tree.tree().elements().map(|(e, _)| *e).collect();

        // New elements (present in `elements` but not in the tree)
        let new_elements = elements_set
            .difference(&tree_elements_set)
            .copied()
            .collect::<Vec<_>>();

        // Missing elements (present in the tree but not in `elements`)
        let missing_elements = tree_elements_set
            .difference(&elements_set)
            .copied()
            .collect::<Vec<_>>();

        // Validate root hash with both insertions and removals
        if tree.tree().root_hash_with(&new_elements, &missing_elements)
            != block.content.state.root_hash
        {
            error!("Fast snapshot chunk root hash mismatch");
            return Ok(());
        }

        let mut batch = smirk::Batch::new();
        let mut block_elements_left_to_find = last_block_elements.clone();

        // Apply insertions that are not part of the last block (the last block
        // will be applied separately via receive_proposal)
        for element in new_elements {
            if element == Element::ZERO {
                continue;
            }
            if let Some(is_add) = last_block_elements.get(&element)
                && *is_add
            {
                // This element is expected to be added by the last block; skip now.
                block_elements_left_to_find.remove(&element);
                continue;
            }
            batch.insert(element, SmirkMetadata::inserted_in(0))?;
        }

        // Apply removals that are not part of the last block (the last block
        // will remove its inputs separately via receive_proposal)
        for element in missing_elements {
            if element == Element::ZERO {
                continue;
            }
            if let Some(is_add) = last_block_elements.get(&element)
                && !*is_add
            {
                // This element is expected to be removed by the last block; skip now.
                block_elements_left_to_find.remove(&element);
                continue;
            }
            batch.remove(element)?;
        }

        if !block_elements_left_to_find.is_empty() {
            error!(
                ?block_elements_left_to_find,
                "Fast snapshot chunk missing elements"
            );
            return Ok(());
        }

        tree.insert_batch(batch)?;
        Ok(())
    }
}

/// An out of sync node sent a snapshot request,
/// if we are in sync, we should send a snapshot offer.
pub(crate) async fn handle_snapshot_request(
    node: &NodeShared,
    peer: PeerId,
    snapshot_id: SnapshotId,
    from_height: BlockHeight,
    _to_height: BlockHeight,
    _kind: SnapshotKind,
) -> Result<(), Error> {
    if node.is_out_of_sync() || from_height > node.height() {
        info!("Ignoring snapshot request, we're too far behind");
        return Ok(());
    }

    info!(?snapshot_id, "Sending snapshot offer");

    let offer = NetworkSnapshotOffer { snapshot_id };
    node.send(peer, NetworkEvent::SnapshotOffer(offer)).await;

    Ok(())
}

/// An out of sync node accepted our snapshot offer,
/// we should send a snapshot chunk.
pub(crate) async fn handle_snapshot_accept(
    node: &NodeShared,
    block_cache: Arc<Mutex<BlockCache>>,
    peer: PeerId,
    snapshot_id: SnapshotId,
    from_height: BlockHeight,
    to_height: BlockHeight,
    kind: SnapshotKind,
) -> Result<(), Error> {
    info!(
        ?snapshot_id,
        ?from_height,
        ?to_height,
        ?kind,
        "Received snapshot accept"
    );

    match kind {
        SnapshotKind::Slow => {
            send_snapshot_chunk_slow(node, block_cache, peer, snapshot_id, from_height, to_height)
                .await
        }
        SnapshotKind::Fast => {
            send_snapshot_chunk_fast(node, peer, snapshot_id, from_height, to_height).await
        }
    }
}

async fn send_snapshot_chunk_slow(
    node: &NodeShared,
    block_cache: Arc<Mutex<BlockCache>>,
    peer: PeerId,
    snapshot_id: SnapshotId,
    from_height: BlockHeight,
    to_height: BlockHeight,
) -> Result<(), Error> {
    let to_height = std::cmp::min(to_height, node.height() + BlockHeight(1));

    let mut blocks = node
        .fetch_blocks(from_height..to_height, BlockListOrder::LowestToHighest)
        .into_iterator()
        .map(|r| r.map(|bf| bf.into_block()))
        .collect::<Result<Vec<_>, NodeError>>()
        .map_err(Box::new)?;

    // Get the latest current pending block
    let pending_proposals = block_cache
        .lock()
        .get_range(from_height, to_height)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    blocks.extend(pending_proposals);

    // Send snapshot chunk
    node.send(
        peer,
        NetworkEvent::SnapshotChunk(SnapshotChunk::Slow(SnapshotChunkSlow {
            snapshot_id,
            chunk: blocks,
        })),
    )
    .await;

    Ok(())
}

async fn send_snapshot_chunk_fast(
    node: &NodeShared,
    peer: PeerId,
    snapshot_id: SnapshotId,
    _from_height: BlockHeight,
    to_height: BlockHeight,
) -> Result<(), Error> {
    let block = node.get_block(to_height).map_err(Box::new)?;

    let elements = node
        .notes_tree()
        .read()
        .tree()
        .elements()
        .filter_map(|(e, meta)| {
            // We can't filter by from_height,
            // because we don't know the height of the elements if they were fast-synced
            if meta.inserted_in <= to_height.0 {
                Some(*e)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Send snapshot chunk
    node.send(
        peer,
        NetworkEvent::SnapshotChunk(SnapshotChunk::Fast(SnapshotChunkFast {
            snapshot_id,
            block: block.map(|b| Box::new(b.into_block())),
            elements,
        })),
    )
    .await;

    Ok(())
}

#[cfg(test)]
#[path = "sync_test.rs"]
mod sync_test;
