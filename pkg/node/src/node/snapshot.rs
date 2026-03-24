use std::sync::Arc;

use contextful::ResultContextExt as _;
use libp2p::PeerId;
use tracing::{info, instrument};

use crate::{
    NodeShared, Result,
    network::{SnapshotChunk, SnapshotKind},
    types::{BlockHeight, SnapshotId},
};

use super::sync;

impl NodeShared {
    /// A node is offering to send us a snapshot
    #[instrument(skip(self))]
    pub(crate) fn receive_snapshot_offer(
        &self,
        peer: PeerId,
        snapshot_id: SnapshotId,
    ) -> Result<()> {
        info!("Received snapshot offer");
        self.sync_worker
            .snapshot_offer(peer, snapshot_id)
            .context("process incoming snapshot offer from peer")?;

        Ok(())
    }

    /// A node is sending us a snapshot chunk
    #[instrument(skip(self))]
    pub(crate) fn receive_snapshot_chunk(&self, peer: PeerId, sc: SnapshotChunk) -> Result<()> {
        info!("Received snapshot chunk");
        self.sync_worker
            .snapshot_chunk(peer, sc)
            .context("process incoming snapshot chunk from peer")?;

        Ok(())
    }

    /// A node is requesting a snapshot from someone
    #[instrument(skip(self))]
    pub(crate) async fn receive_snapshot_request(
        &self,
        peer: PeerId,
        snapshot_id: SnapshotId,
        from_height: BlockHeight,
        to_height: BlockHeight,
        kind: SnapshotKind,
    ) -> Result<()> {
        info!("Received snapshot request");
        sync::handle_snapshot_request(self, peer, snapshot_id, from_height, to_height, kind)
            .await
            .context("handle snapshot request from peer")?;

        Ok(())
    }

    /// A node wants us to send them a snapshot
    #[instrument(skip(self))]
    pub(crate) async fn receive_snapshot_accept(
        &self,
        peer: PeerId,
        id: SnapshotId,
        from_height: BlockHeight,
        to_height: BlockHeight,
        kind: SnapshotKind,
    ) -> Result<()> {
        info!("Received snapshot accept");
        sync::handle_snapshot_accept(
            self,
            Arc::clone(&self.block_cache),
            peer,
            id,
            from_height,
            to_height,
            kind,
        )
        .await
        .context("handle snapshot accept from peer")?;

        Ok(())
    }
}
