use std::fmt::Debug;
use std::sync::Arc;

use block_store::{BlockListOrder, StoreList};
use chrono::{NaiveDate, Utc};
use parking_lot::RwLock;
use tokio_stream::StreamExt;

use crate::block::Block;
use crate::types::BlockHeight;
use crate::{NodeShared, node};

pub struct TodayStats {
    node: Arc<NodeShared>,
    stats: RwLock<(NaiveDate, u64)>,
}

impl Debug for TodayStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TodayStats")
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl TodayStats {
    pub fn new(node: Arc<NodeShared>) -> Self {
        Self {
            node,
            stats: RwLock::new((Utc::now().date_naive(), 0)),
        }
    }

    pub fn count(&self) -> u64 {
        let today = Utc::now().date_naive();
        let (date, count) = *self.stats.read();
        if date == today { count } else { 0 }
    }

    pub async fn run(self: Arc<Self>) {
        let start_height = self.wait_for_start_height().await;

        let last_primed_height = self.prime_stats(start_height);
        let stream_start = last_primed_height
            .map(|h| BlockHeight(h.0 + 1))
            .unwrap_or(start_height);

        let mut stream = self.node.commit_stream(Some(stream_start)).await;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            tokio::select! {
                Some(res) = stream.next() => {
                    match res {
                        Ok(block) => self.process_new_block(block),
                        Err(e) => tracing::error!(?e, "Stream error"),
                    }
                }
                _ = interval.tick() => self.check_day_rollover(),
            }
        }
    }

    fn prime_stats(&self, start_height: BlockHeight) -> Option<BlockHeight> {
        let blocks = self
            .node
            .fetch_blocks_non_empty(start_height.., BlockListOrder::LowestToHighest)
            .into_iterator();

        let mut max_height = None;

        for block in blocks.flatten() {
            let block = Arc::new(block.into_block());
            max_height = Some(block.content.header.height);
            self.process_new_block(block);
        }

        max_height
    }

    async fn wait_for_start_height(&self) -> BlockHeight {
        loop {
            let today = Utc::now().date_naive();
            match self.find_first_non_empty_block_of_day(today) {
                Ok(height) => {
                    *self.stats.write() = (today, 0);
                    return height;
                }
                Err(err) => {
                    tracing::error!(?err, "Failed to scan start height, retrying in 5s");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    fn process_new_block(&self, block: Arc<Block>) {
        let height = block.content.header.height;
        let block_date = self.get_block_date(height);
        let txns_count = block.content.state.txns.len() as u64;

        let mut stats = self.stats.write();
        if block_date > stats.0 {
            *stats = (block_date, txns_count);
        } else if block_date == stats.0 {
            stats.1 += txns_count;
        }
    }

    fn check_day_rollover(&self) {
        let now = Utc::now().date_naive();
        let mut stats = self.stats.write();
        if now > stats.0 {
            *stats = (now, 0);
        }
    }

    fn get_block_date(&self, height: BlockHeight) -> NaiveDate {
        let block_format = self.node.get_block(height).ok().flatten();
        let time = block_format
            .as_ref()
            .and_then(|b| b.metadata().timestamp_unix_s)
            .unwrap_or_else(|| NodeShared::estimate_block_time(height, self.node.max_height()));

        chrono::DateTime::<Utc>::from_timestamp(time as i64, 0)
            .unwrap()
            .date_naive()
    }

    fn find_first_non_empty_block_of_day(
        &self,
        today: NaiveDate,
    ) -> Result<BlockHeight, node::Error> {
        let blocks = self
            .node
            .fetch_blocks_non_empty(.., BlockListOrder::HighestToLowest)
            .into_iterator();

        let max_height = self.node.max_height();
        let mut start_height = BlockHeight(max_height.0 + 1);

        let today_start_ts = chrono::DateTime::<Utc>::from_naive_utc_and_offset(
            today.and_hms_opt(0, 0, 0).unwrap(),
            Utc,
        )
        .timestamp() as u64;

        for block in blocks {
            let block = block?;
            let time = block.metadata().timestamp_unix_s;
            let block = block.into_block();
            let block_height = block.content.header.height;

            let time =
                time.unwrap_or_else(|| NodeShared::estimate_block_time(block_height, max_height));

            if time < today_start_ts {
                break;
            }
            start_height = block_height;
        }
        Ok(start_height)
    }
}
