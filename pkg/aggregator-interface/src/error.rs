use contextful::Contextful;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("[aggregator-interface]: contract error")]
    ContractError(#[from] Contextful<ContractError>),

    #[error("[aggregator-interface]: block prover error")]
    BlockProverError(#[from] Contextful<BlockProverError>),

    #[error("[aggregator-interface]: missing approval block after batch")]
    MissingApprovalBlock,

    #[error("[aggregator-interface]: batch blocks unexpectedly empty")]
    EmptyBatch,

    #[error("[aggregator-interface]: implementation specific error")]
    ImplementationSpecific(Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error("[aggregator-interface]: contract root mismatch")]
    RootMismatch,

    #[error("[aggregator-interface]: invalid proof submitted to contract")]
    InvalidProof,

    #[error("[aggregator-interface]: implementation specific error")]
    ImplementationSpecific(Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, thiserror::Error)]
pub enum BlockProverError {
    #[error("[aggregator-interface]: implementation specific error")]
    ImplementationSpecific(Box<dyn std::error::Error + Send + Sync>),
}
