// lint-long-file-override allow-max-lines=300
mod tree;

pub use tree::get_block_tree;

use super::{State, txn::TxnWithInfo};
use crate::{
    Error, Result,
    block::{
        Block as NodeBlock, BlockContent as NodeBlockContent, BlockHeader as NodeBlockHeader,
        BlockState as NodeBlockState,
    },
    node,
};
use actix_web::web;
use either::Either;
use element::Element;
use primitives::{
    block_height::BlockHeight,
    hash::CryptoHash,
    pagination::{OpaqueCursor, OpaqueCursorChoice, Paginator},
    sig::Signature,
};
use rpc::error::HttpResult;
use serde::{Deserialize, Serialize, de::IntoDeserializer};
use wire_message::WireMessage;

const MAX_BLOCKS_LIMIT: usize = 256;

pub type BlockResponse = BlockWithInfo;

#[derive(Serialize)]
pub struct Block {
    content: BlockContent,
    signature: Signature,
}

impl Block {
    fn from_node_block(block: NodeBlock, time: u64) -> Self {
        let NodeBlock { content, signature } = block;
        let NodeBlockContent { header, state } = content;
        let NodeBlockHeader {
            height,
            last_block_hash,
            epoch_id,
            last_final_block_hash,
            approvals,
        } = header;
        let NodeBlockState { root_hash, txns } = state;

        Self {
            content: BlockContent {
                header: BlockHeader {
                    height,
                    last_block_hash,
                    epoch_id,
                    last_final_block_hash,
                    approvals,
                },
                state: BlockState {
                    root_hash,
                    txns: txns
                        .into_iter()
                        .enumerate()
                        .map(|(index_in_block, proof)| TxnWithInfo {
                            hash: proof.hash(),
                            proof,
                            index_in_block: index_in_block as u64,
                            block_height: height,
                            time,
                        })
                        .collect(),
                },
            },
            signature,
        }
    }
}

#[derive(Serialize)]
struct BlockContent {
    header: BlockHeader,
    state: BlockState,
}

#[derive(Serialize)]
struct BlockHeader {
    height: BlockHeight,
    last_block_hash: CryptoHash,
    epoch_id: u64,
    last_final_block_hash: CryptoHash,
    approvals: Vec<Signature>,
}

#[derive(Serialize)]
struct BlockState {
    root_hash: Element,
    txns: Vec<TxnWithInfo>,
}

#[derive(Debug, Serialize)]
pub enum BlockIdentifier {
    Hash(CryptoHash),
    Height(BlockHeight),
}

impl<'de> Deserialize<'de> for BlockIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        match value {
            value if value.len() == CryptoHash::SIZE * 2 => Ok(BlockIdentifier::Hash(
                CryptoHash::deserialize(value.into_deserializer())?,
            )),
            value => Ok(BlockIdentifier::Height(BlockHeight(
                value.parse().map_err(serde::de::Error::custom)?,
            ))),
        }
    }
}

#[tracing::instrument(err, skip_all)]
pub async fn get_block(
    state: web::Data<State>,
    path: web::Path<(BlockIdentifier,)>,
) -> HttpResult<web::Json<BlockResponse>> {
    tracing::info!(method = "get_block", ?path, "Incoming request");

    let (block_identifier,) = path.into_inner();

    let block = match block_identifier {
        BlockIdentifier::Height(height) => state
            .node
            .get_block(height)?
            .ok_or(Error::BlockNotFound { block: height })?,
        BlockIdentifier::Hash(hash) => state
            .node
            .get_block_by_hash(hash)?
            .ok_or(Error::BlockHashNotFound { block: hash })?,
    };

    let (block, metadata) = match block.upgrade(&mut ()).unwrap() {
        node::BlockFormat::V1(_) => unreachable!("already upgraded"),
        node::BlockFormat::V2(block, metadata) => (block, metadata),
    };

    let max_height = state.node.max_height();

    let time = metadata
        .timestamp_unix_s
        .unwrap_or(node::NodeShared::estimate_block_time(
            block.content.header.height,
            max_height,
        ));
    Ok(web::Json(BlockResponse {
        time,
        hash: block.hash(),
        block: Block::from_node_block(block, time),
    }))
}

#[derive(Serialize)]
pub struct BlockWithInfo {
    block: Block,
    hash: CryptoHash,
    time: u64,
}

#[derive(Serialize)]
pub struct ListBlocksResponse {
    blocks: Vec<BlockWithInfo>,
    cursor: OpaqueCursor<BlockHeight>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ListBlocksOrder {
    LowestToHighest,
    HighestToLowest,
}

impl ListBlocksOrder {
    pub fn highest_to_lowest() -> Self {
        Self::HighestToLowest
    }
}

impl From<ListBlocksOrder> for block_store::BlockListOrder {
    fn from(order: ListBlocksOrder) -> Self {
        match order {
            ListBlocksOrder::LowestToHighest => block_store::BlockListOrder::LowestToHighest,
            ListBlocksOrder::HighestToLowest => block_store::BlockListOrder::HighestToLowest,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListBlocksQuery {
    limit: Option<usize>,
    cursor: Option<OpaqueCursorChoice<BlockHeight>>,
    #[serde(default = "ListBlocksOrder::highest_to_lowest")]
    order: ListBlocksOrder,
    #[serde(default = "bool::default")]
    skip_empty: bool,
}

#[tracing::instrument(err, skip_all)]
pub async fn list_blocks(
    state: web::Data<State>,
    web::Query(query): web::Query<ListBlocksQuery>,
) -> HttpResult<web::Json<ListBlocksResponse>> {
    tracing::info!(method = "list_blocks", ?query, "Incoming request");

    let ListBlocksQuery {
        limit,
        cursor,
        order,
        skip_empty,
    } = query;
    let cursor = cursor.map(|c| c.into_inner());

    let limit = limit.unwrap_or(10).min(MAX_BLOCKS_LIMIT);

    let blocks = if skip_empty {
        Either::Left(
            state
                .node
                .fetch_blocks_non_empty_paginated(&cursor, order.into(), limit)?,
        )
    } else {
        Either::Right(
            state
                .node
                .fetch_blocks_paginated(&cursor, order.into(), limit)?,
        )
    };

    let max_height = state.node.max_height();

    let (cursor, blocks) = Paginator::new(
        blocks.map(|r| {
            let (block, metadata) = match r?.upgrade(&mut ()).unwrap() {
                node::BlockFormat::V1(_) => unreachable!("already upgraded"),
                node::BlockFormat::V2(block, metadata) => (block, metadata),
            };

            let time = metadata
                .timestamp_unix_s
                .unwrap_or(node::NodeShared::estimate_block_time(
                    block.content.header.height,
                    max_height,
                ));
            Ok::<_, node::Error>(BlockWithInfo {
                time,
                hash: block.hash(),
                block: Block::from_node_block(block, time),
            })
        }),
        |r| {
            r.as_ref()
                .ok()
                .map(|block| block.block.content.header.height)
        },
    )
    .collect::<Result<Vec<_>, _>>();

    let blocks = blocks?;

    Ok(web::Json(ListBlocksResponse {
        blocks,
        cursor: cursor.into_opaque(),
    }))
}
