use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
pub use barretenberg_api_interface::{
    BbBackendError, ProveRequest, ProveResponse, ServerError, VerifyRequest, VerifyResponse,
};
use barretenberg_interface::{BbBackend, Result};
use url::Url;

pub mod error;
mod http_transport;

use error::ClientError;
use http_transport::{
    DEFAULT_CONNECT_TIMEOUT, DEFAULT_EXPECT_CONTINUE_TIMEOUT_BUFFER, DEFAULT_MAX_RETRY_DURATION,
    DEFAULT_RETRY_DELAY, HttpTransport,
};

#[async_trait]
pub trait ApiTransport: Send + Sync {
    async fn prove(&self, request: ProveRequest)
    -> std::result::Result<ProveResponse, ClientError>;
    async fn verify(
        &self,
        request: VerifyRequest,
    ) -> std::result::Result<VerifyResponse, ClientError>;
}

#[derive(Clone)]
pub struct ClientBackend {
    transport: Arc<dyn ApiTransport>,
}

impl ClientBackend {
    pub fn new(base_url: Url) -> Result<Self> {
        let transport = HttpTransport::new(
            base_url,
            Duration::from_secs(5 * 60),
            DEFAULT_CONNECT_TIMEOUT,
            Some(Duration::from_millis(100)),
            DEFAULT_RETRY_DELAY,
            DEFAULT_MAX_RETRY_DURATION,
            DEFAULT_EXPECT_CONTINUE_TIMEOUT_BUFFER,
        )
        .map_err(ClientError::from)?;
        Ok(Self::with_transport(transport))
    }

    pub fn with_timeout(base_url: Url, timeout: Duration) -> Result<Self> {
        let transport = HttpTransport::new(
            base_url,
            timeout,
            DEFAULT_CONNECT_TIMEOUT,
            Some(Duration::from_millis(100)),
            DEFAULT_RETRY_DELAY,
            DEFAULT_MAX_RETRY_DURATION,
            DEFAULT_EXPECT_CONTINUE_TIMEOUT_BUFFER,
        )
        .map_err(ClientError::from)?;
        Ok(Self::with_transport(transport))
    }

    pub fn with_timeout_and_permit(
        base_url: Url,
        timeout: Duration,
        permit: Duration,
    ) -> Result<Self> {
        let transport = HttpTransport::new(
            base_url,
            timeout,
            DEFAULT_CONNECT_TIMEOUT,
            Some(permit),
            DEFAULT_RETRY_DELAY,
            DEFAULT_MAX_RETRY_DURATION,
            DEFAULT_EXPECT_CONTINUE_TIMEOUT_BUFFER,
        )
        .map_err(ClientError::from)?;
        Ok(Self::with_transport(transport))
    }

    pub fn with_retry_policy(
        base_url: Url,
        timeout: Duration,
        connect_timeout: Duration,
        permit_timeout: Option<Duration>,
        retry_delay: Duration,
        max_retry_duration: Duration,
        expect_continue_timeout_buffer: Duration,
    ) -> Result<Self> {
        let transport = HttpTransport::new(
            base_url,
            timeout,
            connect_timeout,
            permit_timeout,
            retry_delay,
            max_retry_duration,
            expect_continue_timeout_buffer,
        )
        .map_err(ClientError::from)?;
        Ok(Self::with_transport(transport))
    }

    pub fn with_transport<T>(transport: T) -> Self
    where
        T: ApiTransport + 'static,
    {
        Self {
            transport: Arc::new(transport),
        }
    }
}

#[async_trait]
impl BbBackend for ClientBackend {
    async fn prove(
        &self,
        program: &[u8],
        bytecode: &[u8],
        key: &[u8],
        witness: &[u8],
        oracle: bool,
    ) -> Result<Vec<u8>> {
        let request = ProveRequest {
            program: program.into(),
            bytecode: bytecode.into(),
            key: key.into(),
            witness: witness.into(),
            oracle,
        };

        let response = self.transport.prove(request).await?;
        Ok(response.proof.into_inner())
    }

    async fn verify(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
        key: &[u8],
        oracle: bool,
    ) -> Result<()> {
        let request = VerifyRequest {
            proof: proof.into(),
            public_inputs: public_inputs.into(),
            key: key.into(),
            oracle,
        };

        let response = self.transport.verify(request).await?;
        if response.valid {
            Ok(())
        } else {
            Err(ClientError::Server(ServerError::Backend(
                BbBackendError::VerificationFailed,
            )))?
        }
    }
}
