use std::{num::TryFromIntError, sync::Arc};

use client_http::{
    AuthError, ClientHttp, ClientHttpAuth, Error as HttpError, HttpBody, NoAuth, NoRpcError,
};
use contextful::{Contextful, ResultContextExt};
use element::Element;
use guild_interface::auth::{AuthRequest, AuthResponse};
use http_interface::HttpMetadata;
use parking_lot::Mutex;
use reqwest::{
    Method, Url,
    header::{AUTHORIZATION, HeaderMap, InvalidHeaderValue},
};
use thiserror::Error;
use zk_circuits::circuits::generated::signature::SignatureInput as CircuitSignatureInput;
use zk_circuits::{BbBackend, Prove};
use zk_primitives::Signature;

#[derive(Clone)]
pub struct GuildClientHttpAuth {
    http_client: ClientHttp,
    private_key: Element,
    jwt: Arc<Mutex<Option<String>>>,
    bb_backend: Arc<dyn BbBackend>,
}

#[async_trait::async_trait]
impl ClientHttpAuth for GuildClientHttpAuth {
    async fn get_auth(&self) -> std::result::Result<HeaderMap, AuthError> {
        self.get_auth_headers()
            .await
            .map_err(|err| -> AuthError { err.into() })
    }

    async fn refresh_auth(&self) -> std::result::Result<(), AuthError> {
        self.refresh_jwt()
            .await
            .context("refresh guild auth token")
            .map_err(GuildAuthError::from)
            .map_err(|err| -> AuthError { err.into() })?;
        Ok(())
    }
}

impl GuildClientHttpAuth {
    pub fn new(base_url: Url, private_key: Element, bb_backend: Arc<dyn BbBackend>) -> Self {
        Self {
            http_client: ClientHttp::new(base_url, HeaderMap::default(), NoAuth),
            private_key,
            jwt: Arc::new(Mutex::new(None)),
            bb_backend,
        }
    }

    /// Get the auth headers to be passed to the request
    pub(crate) async fn get_auth_headers(&self) -> Result<HeaderMap, GuildAuthError> {
        let jwt = self.get_jwt().await?;
        let mut headers = HeaderMap::new();
        let authorization = format!("Bearer {jwt}")
            .parse()
            .context("parse Authorization header value")?;
        headers.insert(AUTHORIZATION, authorization);
        Ok(headers)
    }

    /// Get the cached auth token if it exists, otherwise request a fresh one
    async fn get_jwt(&self) -> Result<String, GuildAuthError> {
        if let Some(jwt) = self.jwt.lock().clone() {
            return Ok(jwt);
        }

        self.refresh_jwt()
            .await
            .context("refresh jwt")
            .map_err(GuildAuthError::from)
    }

    /// Refresh the auth JWT and cache the result for future requests
    async fn refresh_jwt(&self) -> Result<String, RefreshJwtError> {
        let private_key = self.private_key;

        let timestamp =
            u64::try_from(chrono::Utc::now().timestamp()).context("convert timestamp to u64")?;

        // Sign proof
        let signature = Signature::new(private_key, Element::new(timestamp));
        let proof = CircuitSignatureInput::from(signature)
            .prove(&*self.bb_backend)
            .await
            .map_err(|err| RefreshJwtError::ProofGeneration {
                message: err.to_string(),
            })?;
        let proof = zk_primitives::SignatureProof::from(proof);

        let request = AuthRequest { proof };
        let response = self
            .http_client
            .post("/auth", Some(HttpBody::json(request)))
            .exec::<NoRpcError>()
            .await
            .context("request auth token")?;

        let auth: AuthResponse = response
            .to_value::<AuthResponse, NoRpcError>()
            .await
            .context("parse auth response")?;

        *self.jwt.lock() = Some(auth.guild.clone());

        Ok(auth.guild)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum GuildAuthError {
    #[error("[guild-client-http/auth] invalid authorization header: {0}")]
    InvalidAuthorization(#[from] Contextful<InvalidHeaderValue>),
    #[error("[guild-client-http/auth] refresh jwt: {0}")]
    Refresh(#[from] Contextful<RefreshJwtError>),
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum RefreshJwtError {
    #[error("[guild-client-http/auth] timestamp conversion: {0}")]
    Timestamp(#[from] Contextful<TryFromIntError>),
    #[error("[guild-client-http/auth] proof generation: {message}")]
    ProofGeneration { message: String },
    #[error("[guild-client-http/auth] http error: {0}")]
    Http(#[from] Contextful<client_http::Error<NoRpcError>>),
}

fn auth_metadata(method: Method, path: &str) -> HttpMetadata {
    HttpMetadata {
        method,
        path: path.to_owned(),
    }
}

impl From<GuildAuthError> for AuthError {
    fn from(err: GuildAuthError) -> Self {
        match err {
            GuildAuthError::InvalidAuthorization(inner) => HttpError::<NoRpcError>::GetAuth(
                inner.to_string(),
                auth_metadata(Method::GET, "/auth/headers"),
            ),
            GuildAuthError::Refresh(inner) => HttpError::<NoRpcError>::RefreshAuth(
                inner.to_string(),
                auth_metadata(Method::POST, "/auth"),
            ),
        }
    }
}
