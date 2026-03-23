use super::State;
use actix_web::web;
use node_interface::HeightResponse;
use rpc::error::HttpResult;

/// GET /height - returns data about the rollup (e.g. root hash, version, etc)

#[tracing::instrument(err, skip(state))]
pub async fn get_height(state: web::Data<State>) -> HttpResult<web::Json<HeightResponse>> {
    Ok(web::Json(HeightResponse {
        height: state.node.height().0,
        root_hash: state.node.root_hash(),
    }))
}
