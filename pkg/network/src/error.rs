use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Missing required network identifier field: {0}")]
    MissingRequiredNetworkIdentifierField(String),
}
