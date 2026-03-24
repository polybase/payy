use super::{State, error};
use actix_web::web;
use rpc::error::HttpResult;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResp {
    height: u64,
}

/// GET /health - returns data about the rollup (e.g. root hash, version, etc)
/// unlike /height, /health will return an error if the node is unhealthy (i.e.
/// out of sync with other nodes)
#[tracing::instrument(skip(state))]
pub async fn get_health(state: web::Data<State>) -> HttpResult<web::Json<HealthResp>> {
    // Out of sync, will trigger service unavailable
    if state.node.is_out_of_sync() {
        return Err(error::Error::OutOfSync)?;
    }

    if state
        .node
        .last_commit_time()
        .is_none_or(|x| x.elapsed().as_secs() > state.health_check_commit_interval_sec)
    {
        return Err(error::Error::OutOfSync)?;
    }

    Ok(web::Json(HealthResp {
        height: state.node.height().0,
    }))
}
