use aggregator_interface::Error;
use node_interface::BlockWithInfo;
use primitives::block_height::BlockHeight;
use std::{io::Error as IoError, time::Duration};
use tokio::time::sleep;

use crate::aggregator::{Aggregator, utils::map_node_error};

#[cfg(test)]
const EMPTY_FETCH_RETRY_DELAY: Duration = Duration::from_millis(1);
#[cfg(not(test))]
const EMPTY_FETCH_RETRY_DELAY: Duration = Duration::from_secs(1);

impl Aggregator {
    pub(super) async fn fetch_blocks(
        &self,
        start_height: u64,
    ) -> Result<Vec<BlockWithInfo>, Error> {
        self.fetch_blocks_exact(start_height, self.block_batch_size)
            .await
    }

    pub(super) async fn fetch_blocks_exact(
        &self,
        start_height: u64,
        limit: usize,
    ) -> Result<Vec<BlockWithInfo>, Error> {
        let mut blocks = Vec::with_capacity(limit);
        let mut next_height = start_height;

        while blocks.len() < limit {
            let remaining = limit - blocks.len();
            let mut fetched = self
                .fetch_blocks_with_limit(next_height, remaining, true)
                .await?;

            if fetched.is_empty() {
                sleep(EMPTY_FETCH_RETRY_DELAY).await;
                continue;
            }

            let last_height = fetched
                .last()
                .map(|block| block.block.content.header.height.0)
                .ok_or_else(|| {
                    Error::ImplementationSpecific(Box::new(IoError::other(
                        "non-empty fetch expected while fetching exact batch",
                    )))
                })?;

            blocks.append(&mut fetched);

            let Some(height_after_last) = last_height.checked_add(1) else {
                return Err(Error::ImplementationSpecific(Box::new(IoError::other(
                    "block height overflow while fetching exact batch",
                ))));
            };
            next_height = height_after_last;
        }

        Ok(blocks)
    }

    pub(super) async fn fetch_blocks_with_limit(
        &self,
        start_height: u64,
        limit: usize,
        skip_empty: bool,
    ) -> Result<Vec<BlockWithInfo>, Error> {
        let response = self
            .node_client
            .blocks(BlockHeight(start_height), limit, skip_empty)
            .await
            .map_err(map_node_error)?;

        Ok(response
            .blocks
            .into_iter()
            .filter(|block| block.block.content.header.height.0 >= start_height)
            .take(limit)
            .collect())
    }

    pub(super) async fn fetch_block(&self, height: u64) -> Result<Option<BlockWithInfo>, Error> {
        let mut blocks = self.fetch_blocks_with_limit(height, 1, false).await?;
        Ok(blocks
            .pop()
            .filter(|block| block.block.content.header.height.0 == height))
    }
}
