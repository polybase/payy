use crate::types::BlockHeight;
use crate::{block::Block, types::SnapshotId};
use borsh::{BorshDeserialize, BorshSerialize};
use derivative::Derivative;
use doomslug::Approval;
use element::Element;
use zk_primitives::UtxoProof;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum NetworkEvent {
    Approval(Approval),
    Block(Block),
    Transaction(UtxoProof),

    /// Request a snapshot from peers.
    SnapshotRequest(SnapshotRequest),

    /// Offer a snapshot to a peer.
    SnapshotOffer(SnapshotOffer),

    /// Accept a snapshot from a peer.
    SnapshotAccept(SnapshotAccept),

    /// A chunk of blocks for the out of sync peer to apply.
    SnapshotChunk(SnapshotChunk),
}

#[derive(Debug, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub enum SnapshotKind {
    Slow,
    Fast,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct SnapshotRequest {
    pub snapshot_id: SnapshotId,
    pub from_height: BlockHeight,
    pub to_height: BlockHeight,
    pub kind: SnapshotKind,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct SnapshotOffer {
    pub snapshot_id: SnapshotId,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct SnapshotAccept {
    pub snapshot_id: SnapshotId,
    pub from_height: BlockHeight,
    pub to_height: BlockHeight,
    pub kind: SnapshotKind,
}

#[derive(Derivative, Clone, BorshSerialize, BorshDeserialize)]
#[derivative(Debug)]
pub struct SnapshotChunkSlow {
    pub snapshot_id: SnapshotId,
    #[derivative(Debug(format_with = "fmt_vec"))]
    pub chunk: Vec<Block>,
}

#[derive(Derivative, Clone, BorshSerialize, BorshDeserialize)]
#[derivative(Debug)]
pub struct SnapshotChunkFast {
    pub snapshot_id: SnapshotId,
    pub block: Option<Box<Block>>,
    /// Elements up to `block`
    #[derivative(Debug(format_with = "fmt_vec"))]
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum SnapshotChunk {
    Slow(SnapshotChunkSlow),
    Fast(SnapshotChunkFast),
}

impl SnapshotChunk {
    pub fn snapshot_id(&self) -> SnapshotId {
        match self {
            SnapshotChunk::Slow(sc) => sc.snapshot_id,
            SnapshotChunk::Fast(sc) => sc.snapshot_id,
        }
    }
}

fn fmt_vec<T>(vec: &[T], fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
    write!(fmt, "Vec(len = {})", vec.len())
}
