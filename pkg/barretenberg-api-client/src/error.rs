use barretenberg_api_interface::ServerError;
use barretenberg_interface::Error as BackendError;
use contextful::Contextful;
use thiserror::Error;

pub use crate::http_transport::error::TransportError;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("[barretenberg-api-client] transport error")]
    // Skipping Contextful because no extra context is needed.
    Transport(#[from] TransportError),

    #[error("[barretenberg-api-client] server error")]
    // Skipping Contextful because no extra context is needed.
    Server(#[from] ServerError),

    #[error("[barretenberg-api-client] tokio runtime creation failed")]
    Runtime(#[from] Contextful<std::io::Error>),
}

impl From<ClientError> for BackendError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::Server(ServerError::Backend(err)) => err.into(),
            ClientError::Transport(TransportError::RetryDeadlineExceeded {
                last_error: Some(err),
                ..
            }) => (*err).into(),
            other => BackendError::ImplementationSpecific(Box::new(other)),
        }
    }
}
