use borsh::{BorshDeserialize, BorshSerialize};
use primitives::block_height::BlockHeight;
use wire_message::WireMessage;

use crate::{TxnFormat, block::Block};

use super::txn_format::TxnMetadata;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BlockMetadata {
    pub timestamp_unix_s: Option<u64>,
}

#[derive(Debug, Clone)]
#[wire_message::wire_message]
pub enum BlockFormat {
    V1(Block),
    V2(Block, BlockMetadata),
    // TODO next version: cache the block hash in metadata
}

impl BlockFormat {
    pub(crate) fn into_block(self) -> Block {
        match self {
            Self::V1(block) => block,
            Self::V2(block, _) => block,
        }
    }

    pub(crate) fn metadata(&self) -> &BlockMetadata {
        match self {
            Self::V1(_) => &BlockMetadata {
                timestamp_unix_s: None,
            },
            Self::V2(_, metadata) => metadata,
        }
    }
}

impl WireMessage for BlockFormat {
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        match self {
            Self::V1(_) => 1,
            Self::V2(_, _) => 2,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        match self {
            Self::V1(block) => Ok(Self::V2(
                block,
                BlockMetadata {
                    timestamp_unix_s: None,
                },
            )),
            Self::V2(_, _) => Err(Self::max_version_error()),
        }
    }
}

impl block_store::Block for BlockFormat {
    type Txn = TxnFormat;

    fn block_height(&self) -> BlockHeight {
        match self {
            Self::V1(block) => block.block_height(),
            Self::V2(block, _) => block.block_height(),
        }
    }

    fn block_hash(&self) -> [u8; 32] {
        match self {
            Self::V1(block) => block.block_hash(),
            Self::V2(block, _) => block.block_hash(),
        }
    }

    fn txns(&self) -> Vec<Self::Txn> {
        match self {
            Self::V1(block) => block.txns(),
            Self::V2(block, block_metadata) => {
                let mut txns = block.txns();
                for TxnFormat::V1(_, metadata) in &mut txns {
                    metadata.block_time = block_metadata.timestamp_unix_s;
                }
                txns
            }
        }
    }
}

impl block_store::Block for Block {
    type Txn = TxnFormat;

    fn block_height(&self) -> BlockHeight {
        self.content.header.height
    }

    fn block_hash(&self) -> [u8; 32] {
        self.hash().into_inner()
    }

    fn txns(&self) -> Vec<Self::Txn> {
        self.content
            .state
            .txns
            .iter()
            .enumerate()
            .map(|(i, t)| {
                TxnFormat::V1(
                    t.clone(),
                    TxnMetadata {
                        block_height: self.block_height(),
                        block_time: None,
                        block_hash: self.block_hash(),
                        block_txn_index: i as u32,
                    },
                )
            })
            .collect()
    }
}
