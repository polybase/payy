use std::time::Duration;

use axum::{
    Json, async_trait,
    body::Body,
    extract::{FromRequest, FromRequestParts},
    http::{Request, header::HeaderMap, request::Parts},
};
use barretenberg_api_interface::PERMIT_TIMEOUT_HEADER;
use contextful::ResultContextExt;
use tokio::sync::OwnedSemaphorePermit;

use crate::{error::HandlerError, server::AppState};

pub(crate) struct Permit(pub OwnedSemaphorePermit);

#[async_trait]
impl FromRequestParts<AppState> for Permit {
    type Rejection = HandlerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let timeout = permit_timeout_from_headers(&parts.headers)?;
        let permit = state.acquire_processing_permit(timeout).await?;
        Ok(Permit(permit))
    }
}

pub(crate) struct CustomJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for CustomJson<T>
where
    Json<T>: FromRequest<S, Rejection = axum::extract::rejection::JsonRejection>,
    S: Send + Sync,
{
    type Rejection = HandlerError;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .context("deserialize request json")?;
        Ok(CustomJson(value))
    }
}

fn permit_timeout_from_headers(headers: &HeaderMap) -> Result<Option<Duration>, HandlerError> {
    match headers.get(PERMIT_TIMEOUT_HEADER) {
        Some(value) => {
            let str_value = value.to_str().context("read permit timeout header")?;
            let parsed = str_value
                .parse::<u64>()
                .context("parse permit timeout header")?;
            Ok(Some(Duration::from_millis(parsed)))
        }
        None => Ok(None),
    }
}
