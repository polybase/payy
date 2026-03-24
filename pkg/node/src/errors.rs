// lint-long-file-override allow-max-lines=250
use std::{error::Error as StdError, fmt, num::ParseIntError, path::PathBuf};

use crate::sync;
use contextful::Contextful;
use element::Element;
use libp2p::PeerId;
use node_interface::RpcError;
use primitives::{block_height::BlockHeight, hash::CryptoHash};

#[derive(Debug)]
pub struct TracingInitError(Box<dyn StdError + Send + Sync>);

impl fmt::Display for TracingInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl StdError for TracingInitError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&*self.0)
    }
}

impl From<Box<dyn StdError + Send + Sync>> for TracingInitError {
    fn from(inner: Box<dyn StdError + Send + Sync>) -> Self {
        Self(inner)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("[node] rpc error: {0}")]
    Rpc(#[from] Contextful<RpcError>),

    #[error("[node] invalid snapshot chunk, peer mismatch - accepted {accepted}, got {got}")]
    SnapshotChunkPeerMismatch {
        accepted: Box<PeerId>,
        got: Box<PeerId>,
    },

    #[error("[node] note already spent: 0x{spent_note:x}")]
    NoteAlreadySpent {
        spent_note: Element,
        failing_txn_hash: Element,
    },

    #[error(
        "[node] leaf 0x{inserted_leaf} was already inserted in the same block in transaction 0x{txn_hash}"
    )]
    LeafAlreadyInsertedInTheSameBlock {
        inserted_leaf: Element,
        txn_hash: Element,
        failing_txn_hash: Element,
    },

    #[error("[node] element is not in any transaction of block {block_height}")]
    ElementNotInTxn {
        element: Element,
        block_height: BlockHeight,
    },

    #[error("[node] block height {block} not found")]
    BlockNotFound { block: BlockHeight },

    #[error("[node] block hash {block} not found")]
    BlockHashNotFound { block: CryptoHash },

    #[error("[node] invalid mint or burn leaves")]
    InvalidMintOrBurnLeaves,

    #[error("[node] invalid mint or burn leaves")]
    InvalidSignature,

    #[error("[node] invalid transaction '{txn}'")]
    InvalidTransaction { txn: Element },

    #[error("[node] invalid block root, got: {got}, expected: {expected}")]
    InvalidBlockRoot { got: Element, expected: Element },

    /// A mint references a chain that is not configured on this node.
    #[error("[node] unsupported chain id {chain_id}")]
    UnsupportedChain { chain_id: u64 },

    #[error("[node] transaction contains locked element {locked_element}")]
    TransactionContainsLockedElement { locked_element: Element },

    #[error("[node] invalid element: {element}")]
    FailedToParseElement {
        element: String,
        #[source]
        source: ParseIntError,
    },

    #[error("[node] invalid hash: {hash}")]
    FailedToParseHash {
        hash: String,
        #[source]
        source: rustc_hex::FromHexError,
    },

    #[error("[node] failed to get eth block number: {0}")]
    FailedToGetEthBlockNumber(#[from] Contextful<web3::Error>),

    #[error("[node] secp256k1 error: {0}")]
    Secp256k1(#[from] Contextful<secp256k1::Error>),

    #[error("[node] web3 secp256k1 error: {0}")]
    Web3Secp256k1(#[from] Contextful<web3::signing::SigningError>),

    #[error("[node] tracing setup error: {0}")]
    Tracing(#[from] Contextful<TracingInitError>),

    #[error("[node] config error: {0}")]
    Config(#[from] Contextful<figment::Error>),

    #[error("[node] doomslug error: {0}")]
    Doomslug(#[from] Contextful<doomslug::Error>),

    #[error("[node] sync error: {0}")]
    Sync(#[from] Contextful<sync::Error>),

    #[error("[node] network error: {0}")]
    Network(#[from] Contextful<p2p2::Error>),

    #[error("[node] block store error: {0}")]
    BlockStore(#[from] Contextful<block_store::Error>),

    #[error("[node] smirk error: {0}")]
    Smirk(#[from] Contextful<smirk::storage::Error>),

    #[error("[node] contracts error: {0}")]
    Contracts(#[from] Contextful<contracts::Error>),

    #[error("[node] io error: {0}")]
    Io(#[from] Contextful<std::io::Error>),

    #[error("[node] unable to resolve home directory for path {path:?}")]
    ConfigMissingHomeDir { path: PathBuf },

    #[error("[node] smirk collision error: {0}")]
    Collision(#[from] Contextful<smirk::CollisionError>),

    #[error("[node] missing primary chain configuration")]
    MissingPrimaryChainConfig,

    #[error("[node] missing rollup contract for primary chain {chain_id}")]
    MissingPrimaryRollupContract { chain_id: u64 },
}
