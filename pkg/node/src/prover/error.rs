use super::db;

use element::Element;
use primitives::block_height::BlockHeight;

use crate::errors::Error as NodeError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to peek the next commit")]
    FailedToPeekNextCommit,

    #[error(
        "prover tree root hash does not match block state root hash, prover tree root hash: {prover_tree}, block state root hash: {block_tree}"
    )]
    ProverTreeRootDoesNotMatchBlockStateRoot {
        prover_tree: Element,
        block_tree: Element,
    },

    #[error("prover skipped over a block {0}, which it was supposed to prove")]
    ProverSkippedBlock(BlockHeight),

    #[error("root {got} does not match expected root {expected}")]
    RootMismatch { got: Element, expected: Element },

    #[error("invalid prover version '{0}'")]
    InvalidProverVersion(u64),

    #[error("failed to get nonce")]
    FailedToGetNonce(#[source] web3::Error),

    #[error("db error")]
    Db(#[from] db::Error),

    #[error("node error")]
    Node(#[from] NodeError),

    #[error("contract error")]
    Contract(#[from] contracts::Error),

    #[error("prover error")]
    Prover(#[from] prover::Error),

    #[error("smirk storage error")]
    SmirkStorage(#[from] smirk::storage::Error),

    #[error("smirk collision error")]
    SmirkCollision(#[from] smirk::CollisionError),

    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error("rocksdb error")]
    RocksdbError(#[from] rocksdb::Error),

    #[error("parse int error")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("tokio mpsc send error")]
    TokioMpscError(#[from] tokio::sync::mpsc::error::SendError<BlockHeight>),

    #[error("tokio-postgres error")]
    TokioPostgresError(#[from] tokio_postgres::Error),
}

pub(super) type Result<T, E = Error> = std::result::Result<T, E>;
