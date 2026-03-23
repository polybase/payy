pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("[barretenberg] backend error: {0}")]
    Backend(String),

    #[error("[barretenberg] verification failed")]
    VerificationFailed,

    #[error("[barretenberg] implementation specific error")]
    ImplementationSpecific(#[source] Box<dyn std::error::Error + Send + Sync>),
}
