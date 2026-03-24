use std::time::Duration;
use tokio::time::error::Elapsed;

use barretenberg_interface::Error as BackendError;
use contextful::Contextful;
use thiserror::Error;

use crate::error::ClientError;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("[barretenberg-api-client] missing base url host")]
    MissingBaseUrlHost,
    #[error("[barretenberg-api-client] missing request url host")]
    MissingRequestUrlHost,
    #[error(
        "[barretenberg-api-client] request timed out after {timeout:?} (attempts: {attempts}): {elapsed}"
    )]
    RequestTimeout {
        timeout: Duration,
        attempts: usize,
        elapsed: Elapsed,
    },
    #[error("[barretenberg-api-client] retry deadline overflow for {duration:?}")]
    RetryDeadlineOverflow { duration: Duration },
    #[error(
        "[barretenberg-api-client] retry deadline exceeded after {duration:?} (attempts: {attempts}) (last error: {last_error:?})"
    )]
    RetryDeadlineExceeded {
        duration: Duration,
        attempts: usize,
        last_error: Option<Box<ClientError>>,
    },
    #[error("[barretenberg-api-client] connection closed while waiting for 100-continue")]
    ConnectionClosedWhileWaitingForContinue,
    #[error("[barretenberg-api-client] timeout waiting for 100-continue")]
    ContinueTimeout,
    #[error("[barretenberg-api-client] connection closed before response")]
    ConnectionClosedBeforeResponse,
    #[error("[barretenberg-api-client] infrastructure error (status: {status}): {body}")]
    Infrastructure { status: u16, body: String },
    #[error("[barretenberg-api-client] unexpected response (status: {status}): {body}")]
    UnexpectedResponse { status: u16, body: String },
    #[error("[barretenberg-api-client] url error: {0}")]
    Url(#[from] Contextful<url::ParseError>),
    #[error("[barretenberg-api-client] dns name error: {0}")]
    DnsName(#[from] Contextful<rustls::pki_types::InvalidDnsNameError>),
    #[error("[barretenberg-api-client] tls error: {0}")]
    Tls(#[from] Contextful<rustls::Error>),
    #[error("[barretenberg-api-client] io error: {0}")]
    Io(#[from] Contextful<std::io::Error>),
    #[error("[barretenberg-api-client] json error: {0}")]
    Json(#[from] Contextful<serde_json::Error>),
    #[error("[barretenberg-api-client] parse error: {0}")]
    Parse(#[from] Contextful<httparse::Error>),
}

impl From<TransportError> for BackendError {
    fn from(error: TransportError) -> Self {
        BackendError::ImplementationSpecific(Box::new(error))
    }
}
