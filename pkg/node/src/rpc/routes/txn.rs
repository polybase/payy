// lint-long-file-override allow-max-lines=600
use super::State;
use crate::{BlockFormat, Error, node};
use actix_web::web;
use block_store::BlockListOrder;
use element::Element;
use futures::StreamExt;
use itertools::Itertools;
use json_with_logging::JsonWithLogging;
use node_interface::{ElementData, RpcError, TransactionRequest, TransactionResponse};
use primitives::{
    block_height::BlockHeight,
    pagination::{Cursor, CursorChoice, OpaqueCursor, OpaqueCursorChoice, Paginator},
};
use rpc::error::{HTTPError, HttpResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wire_message::WireMessage;
use zk_circuits::{Proof, Verify};
use zk_primitives::UtxoProof;

#[tracing::instrument(err, skip_all)]
pub async fn submit_txn(
    state: web::Data<State>,
    body: JsonWithLogging<TransactionRequest>,
) -> HttpResult<web::Json<TransactionResponse>> {
    let data = body.into_inner();
    let utxo_proof = data.proof;

    tracing::info!(
        method = "submit_txn",
        proof = serde_json::to_string(&utxo_proof).unwrap(),
        "Incoming request"
    );

    let circuit_proof = Proof::from(utxo_proof.clone());
    if let Err(_err) = circuit_proof.verify(&*state.node.bb_backend).await {
        return Err(RpcError::InvalidProof)?;
    }

    let utxo_hash = utxo_proof.hash();

    if let Some((_, metadata)) = state
        .node
        .get_txn(utxo_hash.to_be_bytes())
        .map_err(HTTPError::from)?
    {
        let root_hash = state
            .node
            .get_block(metadata.block_height)
            .map_err(HTTPError::from)?
            .map(|block_format| match block_format {
                node::BlockFormat::V1(block) => block.content.state.root_hash,
                node::BlockFormat::V2(block, _) => block.content.state.root_hash,
            })
            .unwrap_or_else(|| {
                tracing::warn!(
                    ?utxo_hash,
                    height = ?metadata.block_height,
                    "missing block for already included transaction"
                );
                Element::ZERO
            });

        return Err(RpcError::TxnAlreadyIncluded {
            root_hash,
            txn_hash: utxo_hash,
            height: metadata.block_height,
        })?;
    }

    let node = Arc::clone(&state.node);
    let block = tokio::spawn(async move { node.submit_transaction_and_wait(utxo_proof).await })
        .await
        .map_err(|err| HTTPError::internal(Box::new(err)))??;

    Ok(web::Json(TransactionResponse {
        height: block.content.header.height,
        root_hash: block.content.state.root_hash,
        txn_hash: utxo_hash,
    }))
}

#[derive(Serialize)]
pub(crate) struct TxnWithInfo {
    pub(crate) proof: UtxoProof,
    pub(crate) index_in_block: u64,
    pub(crate) hash: Element,
    pub(crate) block_height: BlockHeight,
    pub(crate) time: u64,
}

#[derive(Serialize)]
pub struct ListTxnsResponse {
    txns: Vec<TxnWithInfo>,
    cursor: OpaqueCursor<ListTxnsPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ListTxnsPosition {
    block: BlockHeight,
    txn: u64,
}

#[derive(Debug, Deserialize)]
enum ListTxnOrder {
    NewestToOldest,
    OldestToNewest,
}

impl ListTxnOrder {
    fn to_block_list_order(&self) -> BlockListOrder {
        match self {
            ListTxnOrder::NewestToOldest => BlockListOrder::HighestToLowest,
            ListTxnOrder::OldestToNewest => BlockListOrder::LowestToHighest,
        }
    }

    fn newest_to_oldest() -> Self {
        ListTxnOrder::NewestToOldest
    }
}

#[derive(Debug, Deserialize)]
pub struct ListTxnsQuery {
    limit: Option<usize>,
    cursor: Option<OpaqueCursorChoice<ListTxnsPosition>>,
    #[serde(default = "ListTxnOrder::newest_to_oldest")]
    order: ListTxnOrder,
    #[serde(default = "bool::default")]
    poll: bool,
}

#[tracing::instrument(err, skip_all)]
pub async fn list_txns(
    state: web::Data<State>,
    path: web::Path<()>,
    web::Query(query): web::Query<ListTxnsQuery>,
) -> HttpResult<web::Json<ListTxnsResponse>> {
    tracing::info!(method = "list_txns", ?path, ?query, "Incoming request");

    let make_block_fetcher = |s: web::Data<State>| {
        move |cursor: &Option<CursorChoice<BlockHeight>>,
              order: BlockListOrder,
              limit: usize|
              -> Result<std::vec::IntoIter<Result<BlockFormat, Error>>, Error> {
            let iter = s
                .node
                .fetch_blocks_non_empty_paginated(cursor, order, limit)?;
            Ok(iter.collect::<Vec<_>>().into_iter())
        }
    };

    let max_height = state.node.max_height();

    let (cursor, transactions) =
        list_txns_inner(make_block_fetcher(state.clone()), &query, max_height)?;

    let (cursor, transactions) = if transactions.is_empty() && query.poll {
        let towards_newer_height = match (&query.order, query.cursor.as_deref()) {
            (ListTxnOrder::NewestToOldest, Some(CursorChoice::Before(before))) => {
                Some(before.inner().block.next())
            }
            (ListTxnOrder::OldestToNewest, Some(CursorChoice::After(after))) => {
                Some(after.inner().block.next())
            }
            _ => None,
        };

        match towards_newer_height {
            None => {
                // There is no new block to wait for,
                // so we just sleep in case the client retries immediately.
                tokio::time::sleep(std::time::Duration::from_secs(25)).await;
                (cursor, transactions)
            }
            Some(height) => {
                let commit_stream = state.node.commit_stream(Some(height)).await;
                let mut non_empty_block_stream = Box::pin(commit_stream.filter(|r| {
                    let error_or_block_has_commits = r
                        .as_ref()
                        .map_or(true, |commit| !commit.content.state.txns.is_empty());

                    async move { error_or_block_has_commits }
                }));

                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(50)) => {
                        (cursor, transactions)
                    }
                    _ = non_empty_block_stream.next() => {
                        list_txns_inner(
                            make_block_fetcher(state.clone()),
                            &query,
                            max_height,
                        )?
                    }
                }
            }
        }
    } else {
        (cursor, transactions)
    };

    Ok(web::Json(ListTxnsResponse {
        cursor: cursor.into_opaque(),
        txns: transactions,
    }))
}

#[allow(clippy::type_complexity)]
fn list_txns_inner<I: Iterator<Item = Result<BlockFormat, node::Error>>>(
    block_fetcher: impl FnOnce(
        &Option<CursorChoice<BlockHeight>>,
        BlockListOrder,
        usize,
    ) -> Result<I, node::Error>,
    query: &ListTxnsQuery,
    max_height: BlockHeight,
) -> Result<(Cursor<ListTxnsPosition>, Vec<TxnWithInfo>), HTTPError> {
    let txn_limit = query.limit.unwrap_or(10).min(100);

    let blocks = block_fetcher(
        &query
            .cursor
            .as_ref()
            .map(|pag| pag.map_pos(|pos| pos.block)),
        query.order.to_block_list_order(),
        // Because we filter later in the code, we need to fetch an extra block
        txn_limit + 1,
    )?;

    let transactions = blocks
        .map(|r| {
            r.map(|r| {
                let (block, metadata) = match r.upgrade(&mut ()).unwrap() {
                    node::BlockFormat::V1(_) => unreachable!("already upgraded"),
                    node::BlockFormat::V2(block, metadata) => (block, metadata),
                };

                block
                    .content
                    .state
                    .txns
                    .into_iter()
                    .enumerate()
                    .map(move |(i, txn)| TxnWithInfo {
                        hash: txn.hash(),
                        proof: txn,
                        index_in_block: i as u64,
                        block_height: block.content.header.height,
                        time: metadata.timestamp_unix_s.unwrap_or(
                            node::NodeShared::estimate_block_time(
                                block.content.header.height,
                                max_height,
                            ),
                        ),
                    })
            })
        })
        .flatten_ok();

    // These are transactions that were returned on the previous page,
    // since the last returned block could have had
    // more (in total at the time) transactions than the limit.
    let txns_to_skip = query
        .cursor
        .clone()
        .map(|cursor| cursor.into_inner())
        .map(|cursor| (cursor.inner().block, 0)..=(cursor.inner().block, cursor.inner().txn));

    let transactions = transactions
        .filter(|r| {
            if let Some(txns_to_skip) = &txns_to_skip {
                let Ok(txn) = r else {
                    return true;
                };

                !txns_to_skip.contains(&(txn.block_height, txn.index_in_block))
            } else {
                true
            }
        })
        .take(txn_limit);

    let (cursor, transactions) = Paginator::new(transactions, |r| {
        r.as_ref().ok().map(|txn| ListTxnsPosition {
            block: txn.block_height,
            txn: txn.index_in_block,
        })
    })
    .collect::<Result<Vec<_>, _>>();

    let transactions = transactions?;

    Ok((
        Cursor {
            // See the comment on txns_to_skip as to why this needs to be inclusive
            before: cursor.before.map(|b| b.inclusive()),
            after: cursor.after.map(|a| a.inclusive()),
        },
        transactions,
    ))
}

#[derive(Serialize)]
pub struct GetTxnResponse {
    txn: TxnWithInfo,
}

#[tracing::instrument(err, skip_all)]
pub async fn get_txn(
    state: web::Data<State>,
    path: web::Path<(Element,)>,
) -> HttpResult<web::Json<GetTxnResponse>> {
    tracing::info!(method = "get_txn", ?path, "Incoming request");

    let (txn_hash,) = path.into_inner();

    let (txn, metadata) = state
        .node
        .get_txn(txn_hash.to_be_bytes())?
        .ok_or(RpcError::TxnNotFound(ElementData { element: txn_hash }))?;

    let time = metadata.block_time.unwrap_or_else(|| {
        node::NodeShared::estimate_block_time(metadata.block_height, state.node.max_height())
    });

    Ok(web::Json(GetTxnResponse {
        txn: TxnWithInfo {
            proof: txn,
            index_in_block: metadata.block_txn_index as u64,
            hash: txn_hash,
            block_height: metadata.block_height,
            time,
        },
    }))
}

#[cfg(test)]
mod tests {
    use contextful::ResultContextExt;
    use primitives::pagination::Opaque;

    use crate::{Block, BlockFormat};

    use super::*;

    #[test]
    fn list_txns_pagination() {
        let tempdir = tempdir::TempDir::new("list_txns").unwrap();

        let store = block_store::BlockStore::<BlockFormat>::create_or_load(tempdir.path()).unwrap();

        let new_block = |height: u64, txns: Vec<UtxoProof>| {
            let mut block = Block::default();
            block.content.header.height = BlockHeight(height);
            block.content.state.txns = txns;
            block
        };

        let new_proof = || UtxoProof::default();

        let blocks = [
            new_block(1, vec![]),
            new_block(2, vec![new_proof()]),
            new_block(3, vec![new_proof(), new_proof()]),
            new_block(4, vec![new_proof()]),
        ];

        let max_height = blocks.last().unwrap().content.header.height;

        for block in &blocks {
            store.set(&BlockFormat::V1(block.clone())).unwrap();
        }

        let block_fetcher =
            |cursor: &Option<CursorChoice<BlockHeight>>, order: BlockListOrder, limit: usize| {
                let blocks = store
                    .list_paginated(cursor, order, limit)
                    .context("list paginated blocks")?
                    .map(|res| -> Result<_, node::Error> {
                        let (_, block) = res.context("load paginated block")?;
                        Ok(block)
                    })
                    .collect::<Vec<_>>();

                Ok(blocks.into_iter())
            };

        let (_pagination, txns) = list_txns_inner(
            block_fetcher,
            &ListTxnsQuery {
                limit: Some(10),
                cursor: None,
                order: ListTxnOrder::NewestToOldest,
                poll: false,
            },
            max_height,
        )
        .unwrap();
        assert_eq!(txns.len(), 4);

        // Newest to oldest
        {
            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: None,
                    order: ListTxnOrder::NewestToOldest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 1);
            assert_eq!(txns[0].block_height, BlockHeight(4));

            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: Some(Opaque(CursorChoice::After(cursor.after.unwrap()))),
                    order: ListTxnOrder::NewestToOldest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 1);
            assert_eq!(txns[0].block_height, BlockHeight(3));

            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: Some(Opaque(CursorChoice::After(cursor.after.unwrap()))),
                    order: ListTxnOrder::NewestToOldest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 1);
            assert_eq!(txns[0].block_height, BlockHeight(3));

            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: Some(Opaque(CursorChoice::After(cursor.after.unwrap()))),
                    order: ListTxnOrder::NewestToOldest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 1);
            assert_eq!(txns[0].block_height, BlockHeight(2));

            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: Some(Opaque(CursorChoice::After(cursor.after.unwrap()))),
                    order: ListTxnOrder::NewestToOldest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 0);
            assert_eq!(cursor.before, None);
            assert_eq!(cursor.after, None);
        };

        // Oldest to newest
        {
            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: None,
                    order: ListTxnOrder::OldestToNewest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 1);
            assert_eq!(txns[0].block_height, BlockHeight(2));

            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: Some(Opaque(CursorChoice::After(cursor.after.unwrap()))),
                    order: ListTxnOrder::OldestToNewest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 1);
            assert_eq!(txns[0].block_height, BlockHeight(3));

            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: Some(Opaque(CursorChoice::After(cursor.after.unwrap()))),
                    order: ListTxnOrder::OldestToNewest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 1);
            assert_eq!(txns[0].block_height, BlockHeight(3));

            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: Some(Opaque(CursorChoice::After(cursor.after.unwrap()))),
                    order: ListTxnOrder::OldestToNewest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 1);
            assert_eq!(txns[0].block_height, BlockHeight(4));

            let (cursor, txns) = list_txns_inner(
                block_fetcher,
                &ListTxnsQuery {
                    limit: Some(1),
                    cursor: Some(Opaque(CursorChoice::After(cursor.after.unwrap()))),
                    order: ListTxnOrder::OldestToNewest,
                    poll: false,
                },
                max_height,
            )
            .unwrap();
            assert_eq!(txns.len(), 0);
            assert_eq!(cursor.before, None);
            assert_eq!(cursor.after, None);
        };
    }
}
