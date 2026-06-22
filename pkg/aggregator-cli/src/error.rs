use aggregator_interface::{BlockProverError, Error as AggregatorError};
use barretenberg_interface::Error as BarretenbergError;
use contextful::Contextful;
use element::Element;
use node_client_http::Error as NodeError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("[aggregator-cli] invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("[aggregator-cli] invalid secret key: {0}")]
    InvalidSecretKey(String),
    #[error(
        "[aggregator-cli] rollup tree mismatch (node {node_root:?} vs contract {contract_root:?})"
    )]
    RootMismatch {
        node_root: Element,
        contract_root: Element,
    },
    #[error("[aggregator-cli] block prover error")]
    BlockProver(#[from] Contextful<BlockProverError>),
    #[error("[aggregator-cli] aggregator error")]
    Aggregator(#[from] Contextful<AggregatorError>),
    #[error("[aggregator-cli] contracts error")]
    Contracts(#[from] Contextful<contracts::Error>),
    #[error("[aggregator-cli] node rpc error")]
    Node(#[from] Contextful<NodeError>),
    #[error("[aggregator-cli] barretenberg backend error")]
    Barretenberg(#[from] Contextful<BarretenbergError>),
    #[error("[aggregator-cli] url parse error")]
    Url(#[from] Contextful<url::ParseError>),
    #[error("[aggregator-cli] tokio join error: {0}")]
    TokioJoin(#[from] Contextful<tokio::task::JoinError>),
}
