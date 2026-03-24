use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use barretenberg_api_interface::ServerError;
use barretenberg_interface::BbBackend;
use thiserror::Error;
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore},
    time,
};

use crate::handlers;

#[derive(Clone)]
pub(crate) struct AppState {
    backend: Arc<dyn BbBackend>,
    processing_limit: Arc<Semaphore>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum PermitAcquireError {
    #[error("[barretenberg-api-server] permit semaphore closed")]
    Closed,
    #[error("[barretenberg-api-server] permit acquire timed out")]
    Timeout,
}

impl AppState {
    pub(crate) fn new(backend: Arc<dyn BbBackend>) -> Self {
        Self {
            backend,
            processing_limit: Arc::new(Semaphore::new(1)),
        }
    }

    pub(crate) fn backend(&self) -> Arc<dyn BbBackend> {
        Arc::clone(&self.backend)
    }

    pub(crate) async fn acquire_processing_permit(
        &self,
        timeout: Option<Duration>,
    ) -> Result<OwnedSemaphorePermit, PermitAcquireError> {
        match timeout {
            Some(duration) => {
                time::timeout(duration, self.processing_limit.clone().acquire_owned())
                    .await
                    .map_err(|_| PermitAcquireError::Timeout)?
                    .map_err(|_| PermitAcquireError::Closed)
            }
            None => self
                .processing_limit
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| PermitAcquireError::Closed),
        }
    }
}

pub fn build_app(backend: Arc<dyn BbBackend>) -> Router {
    // agg_final is 28MB. For future-proofing, allowing up to 64MB.
    let limit_layer = DefaultBodyLimit::max(64 * 1024 * 1024);

    Router::new()
        .route("/prove", post(handlers::prove))
        .route("/verify", post(handlers::verify))
        .route("/health", get(healthcheck))
        .layer(limit_layer)
        .with_state(AppState::new(backend))
        .fallback(api_fallback)
}

async fn healthcheck() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn api_fallback() -> impl IntoResponse {
    let err = ServerError::InvalidRequest {
        message: "not found".to_owned(),
    };
    (err.status_code(), Json(err))
}
