use crate::{Error, Result, rpc::routes::State};
use actix_web::web;
use block_store::{ElementHistoryIndexEntry, ElementHistoryKind};
use element::Element;
use node_interface::{BlockTreeDiffChanges, BlockTreeResponse};
use primitives::block_height::BlockHeight;
use rpc::error::HttpResult;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
pub struct BlockTreeQuery {
    #[serde(default)]
    pub diff_from: Option<BlockHeight>,
}

#[tracing::instrument(err, skip(state))]
pub async fn get_block_tree(
    state: web::Data<State>,
    path: web::Path<(BlockHeight,)>,
    query: web::Query<BlockTreeQuery>,
) -> HttpResult<web::Json<BlockTreeResponse>> {
    let (height,) = path.into_inner();
    let query = query.into_inner();
    tracing::info!(
        method = "get_block_tree",
        ?height,
        ?query,
        "Incoming request"
    );

    let block = state
        .node
        .get_block(height)?
        .ok_or(Error::BlockNotFound { block: height })?
        .into_block();
    let root_hash = block.content.state.root_hash;

    let diff = query
        .diff_from
        .map(|from_height| {
            if from_height >= height {
                Ok(BlockTreeDiffChanges {
                    from_height,
                    additions: vec![],
                    removals: vec![],
                })
            } else {
                compute_tree_diff(state.as_ref(), from_height, height)
            }
        })
        .transpose()?;
    let elements = if diff.is_some() {
        None
    } else {
        Some(
            collect_tree_elements(state.as_ref(), height)?
                .into_iter()
                .collect(),
        )
    };

    Ok(web::Json(BlockTreeResponse {
        height,
        root_hash,
        elements,
        diff,
    }))
}

fn collect_tree_elements(state: &State, height: BlockHeight) -> Result<BTreeSet<Element>> {
    let mut elements = BTreeSet::new();
    for entry in state.node.element_history_range(..=height) {
        apply_history_entry(&mut elements, entry);
    }

    Ok(elements)
}

fn compute_tree_diff(
    state: &State,
    from_height: BlockHeight,
    to_height: BlockHeight,
) -> Result<BlockTreeDiffChanges> {
    let mut additions = BTreeSet::new();
    let mut removals = BTreeSet::new();
    for entry in state
        .node
        .element_history_range(from_height.next()..=to_height)
    {
        apply_diff_entry(&mut additions, &mut removals, entry);
    }

    Ok(BlockTreeDiffChanges {
        from_height,
        additions: additions.into_iter().collect(),
        removals: removals.into_iter().collect(),
    })
}

fn apply_history_entry(target: &mut BTreeSet<Element>, entry: ElementHistoryIndexEntry) {
    if entry.element.is_zero() {
        return;
    }

    match entry.kind {
        ElementHistoryKind::Output => {
            target.insert(entry.element);
        }
        ElementHistoryKind::Input => {
            target.remove(&entry.element);
        }
    }
}

fn apply_diff_entry(
    additions: &mut BTreeSet<Element>,
    removals: &mut BTreeSet<Element>,
    entry: ElementHistoryIndexEntry,
) {
    if entry.element.is_zero() {
        return;
    }

    match entry.kind {
        ElementHistoryKind::Output => {
            if !removals.remove(&entry.element) {
                additions.insert(entry.element);
            }
        }
        ElementHistoryKind::Input => {
            if !additions.remove(&entry.element) {
                removals.insert(entry.element);
            }
        }
    }
}
