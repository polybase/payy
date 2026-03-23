use super::{State, error};
use actix_web::web;
use element::Element;
use rpc::error::HttpResult;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct MerklePathRequestQuery {
    commitments: String,
}

#[derive(Serialize)]
pub struct MerklePathResponse {
    paths: Vec<Vec<Element>>,
}

#[tracing::instrument(err, skip_all)]
pub async fn get_merkle_paths(
    state: web::Data<State>,
    query: web::Query<MerklePathRequestQuery>,
) -> HttpResult<web::Json<MerklePathResponse>> {
    tracing::info!(method = "get_merkle_paths", ?query, "Incoming request");

    let commitments = query
        .0
        .commitments
        .split(',')
        .map(|c| {
            Element::from_str(c)
                .map_err(|e| error::Error::InvalidElement(c.to_string(), e))
                .map_err(rpc::error::HTTPError::from)
        })
        .collect::<HttpResult<Vec<Element>>>()?;

    let paths = state.node.get_merkle_paths(&commitments)?;
    Ok(web::Json(MerklePathResponse { paths }))
}
