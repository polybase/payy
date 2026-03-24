use actix_web::web;
use element::Element;
use rpc::error::HttpResult;
use serde::Serialize;

use crate::rpc::routes::State;

#[derive(Serialize)]
pub struct SmirkElementInfo {
    element: Element,
    inserted_at_height: u64,
}

pub type GetAllSmirkElementsResponse = Vec<SmirkElementInfo>;

#[tracing::instrument(err, skip_all)]
pub async fn get_all_smirk_elements(
    state: web::Data<State>,
) -> HttpResult<web::Json<GetAllSmirkElementsResponse>> {
    tracing::info!(method = "get_all_smirk_elements", "Incoming request");

    let notes_tree = state.node.notes_tree().read();
    let elements = notes_tree
        .tree()
        .elements()
        .map(|(element, metadata)| SmirkElementInfo {
            element: *element,
            inserted_at_height: metadata.inserted_in,
        })
        .collect();

    Ok(web::Json(elements))
}
