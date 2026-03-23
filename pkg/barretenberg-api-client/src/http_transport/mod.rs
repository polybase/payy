// lint-long-file-override allow-max-lines=400
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use barretenberg_api_interface::ServerError;
use contextful::ResultContextExt;
use rustls::pki_types::ServerName;
use serde::{Serialize, de::DeserializeOwned};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_rustls::{TlsConnector, rustls};
use tracing::{debug, warn};
use url::Url;

use crate::error::ClientError;
use crate::{ApiTransport, ProveRequest, ProveResponse, VerifyRequest, VerifyResponse};

pub(crate) mod error;
mod protocol;
pub(crate) mod stream;

use error::TransportError;
use protocol::{perform_handshake_and_write_body, read_and_parse_response};
use stream::MaybeTlsStream;

pub(crate) const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(500);
pub(crate) const DEFAULT_MAX_RETRY_DURATION: Duration = Duration::from_secs(60 * 60);
pub(crate) const DEFAULT_EXPECT_CONTINUE_TIMEOUT_BUFFER: Duration = Duration::from_millis(500);
pub(crate) const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

pub(crate) struct HttpTransport {
    base_url: Url,
    timeout: Duration,
    connect_timeout: Duration,
    permit_timeout: Option<Duration>,
    retry_delay: Duration,
    max_retry_duration: Duration,
    expect_continue_timeout_buffer: Duration,
    tls_connector: TlsConnector,
}

impl HttpTransport {
    pub(crate) fn new(
        base_url: Url,
        timeout: Duration,
        connect_timeout: Duration,
        permit_timeout: Option<Duration>,
        retry_delay: Duration,
        max_retry_duration: Duration,
        expect_continue_timeout_buffer: Duration,
    ) -> std::result::Result<Self, TransportError> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder_with_provider(
            rustls::crypto::ring::default_provider().into(),
        )
        .with_safe_default_protocol_versions()
        .context("configure tls protocol versions")?
        .with_root_certificates(root_store)
        .with_no_client_auth();
        let tls_connector = TlsConnector::from(Arc::new(config));

        Ok(Self {
            base_url,
            timeout,
            connect_timeout,
            permit_timeout,
            retry_delay,
            max_retry_duration,
            expect_continue_timeout_buffer,
            tls_connector,
        })
    }

    async fn connect(&self) -> std::result::Result<MaybeTlsStream, ClientError> {
        match tokio::time::timeout(self.connect_timeout, self.connect_inner()).await {
            Ok(result) => result,
            Err(elapsed) => Err(ClientError::Transport(TransportError::RequestTimeout {
                timeout: self.connect_timeout,
                attempts: 0,
                elapsed,
            })),
        }
    }

    async fn connect_inner(&self) -> std::result::Result<MaybeTlsStream, ClientError> {
        let host = self
            .base_url
            .host_str()
            .ok_or_else(|| TransportError::MissingBaseUrlHost)?;
        let port =
            self.base_url
                .port_or_known_default()
                .unwrap_or(if self.base_url.scheme() == "https" {
                    443
                } else {
                    80
                });
        let addr = format!("{}:{}", host, port);

        let stream = TcpStream::connect(&addr)
            .await
            .context("connect to barretenberg api host")
            .map_err(TransportError::Io)?;

        let _ = stream.set_nodelay(true);

        if self.base_url.scheme() == "https" {
            let domain = ServerName::try_from(host)
                .context("parse dns name")
                .map_err(TransportError::DnsName)?
                .to_owned();

            let connector = self.tls_connector.clone();
            let stream = connector
                .connect(domain, stream)
                .await
                .context("perform tls handshake")
                .map_err(TransportError::Io)?;

            Ok(MaybeTlsStream::Tls(Box::new(stream)))
        } else {
            Ok(MaybeTlsStream::Plain(stream))
        }
    }

    async fn send_request<Req, Resp>(
        &self,
        path: &str,
        payload: Req,
        permit_timeout: Option<Duration>,
    ) -> std::result::Result<Resp, ClientError>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        let mut attempt = 0;
        let mut last_error: Option<ClientError> = None;
        let deadline = Instant::now().checked_add(self.max_retry_duration).ok_or(
            TransportError::RetryDeadlineOverflow {
                duration: self.max_retry_duration,
            },
        )?;

        loop {
            attempt += 1;

            let result = tokio::time::timeout(
                self.timeout,
                self.try_request(path, &payload, permit_timeout),
            )
            .await;

            let result = match result {
                Ok(res) => res,
                Err(err) => Err(ClientError::Transport(TransportError::RequestTimeout {
                    timeout: self.timeout,
                    attempts: attempt,
                    elapsed: err,
                })),
            };

            let err = match result {
                Ok(resp) => return Ok(resp),
                Err(err) => err,
            };

            match &err {
                ClientError::Server(ServerError::ServiceUnavailable { .. }) => {
                    debug!(
                        target: "barretenberg_api_client",
                        attempt = attempt,
                        ?last_error,
                        "service unavailable, retrying"
                    );
                }
                ClientError::Server(_) => return Err(err),
                ClientError::Transport(transport_err) => {
                    warn!(
                        target: "barretenberg_api_client",
                        error = %transport_err,
                        attempt = attempt,
                        ?last_error,
                        "received transport error, retrying"
                    );
                }
                _ => return Err(err),
            }

            last_error = Some(err);

            if Instant::now() >= deadline {
                return Err(ClientError::Transport(
                    TransportError::RetryDeadlineExceeded {
                        duration: self.max_retry_duration,
                        attempts: attempt,
                        last_error: last_error.map(Box::new),
                    },
                ));
            }

            sleep(self.retry_delay).await;
        }
    }

    async fn try_request<Req, Resp>(
        &self,
        path: &str,
        payload: &Req,
        permit_timeout: Option<Duration>,
    ) -> std::result::Result<Resp, ClientError>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        let body = serde_json::to_vec(payload)
            .context("serialize payload")
            .map_err(TransportError::Json)?;

        let mut stream = self.connect().await?;

        let buffer = perform_handshake_and_write_body(
            &mut stream,
            &self.base_url,
            path,
            &body,
            permit_timeout,
            self.expect_continue_timeout_buffer,
        )
        .await?;

        read_and_parse_response(&mut stream, buffer).await
    }
}

#[async_trait]
impl ApiTransport for HttpTransport {
    async fn prove(
        &self,
        request: ProveRequest,
    ) -> std::result::Result<ProveResponse, ClientError> {
        self.send_request("prove", request, self.permit_timeout)
            .await
    }

    async fn verify(
        &self,
        request: VerifyRequest,
    ) -> std::result::Result<VerifyResponse, ClientError> {
        self.send_request("verify", request, None).await
    }
}
