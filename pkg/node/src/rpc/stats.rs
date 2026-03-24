use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, atomic::AtomicBool},
};

use block_store::{BlockListOrder, StoreList};
use chrono::{NaiveDate, Utc};
use parking_lot::RwLock;

use crate::{NodeShared, node};

use super::stats_today::TodayStats;

pub struct TxnStats {
    node: Arc<NodeShared>,
    ready: Arc<AtomicBool>,
    last_7_days: Arc<RwLock<Vec<(chrono::NaiveDate, u64)>>>,
    today: Arc<TodayStats>,
}

impl Debug for TxnStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TxnStats")
            .field("ready", &self.ready)
            .field("last_7_days", &self.last_7_days)
            .field("today", &self.today)
            .finish()
    }
}

impl TxnStats {
    pub fn new(node: Arc<NodeShared>) -> Self {
        Self {
            node: Arc::clone(&node),
            last_7_days: Arc::new(RwLock::new(Vec::new())),
            today: Arc::new(TodayStats::new(node)),
            ready: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn last_7_days(&self) -> Vec<(NaiveDate, u64)> {
        self.last_7_days.read().clone()
    }

    fn refresh_last_7_days(&self, last_day: NaiveDate) -> Result<(), node::Error> {
        let blocks = self
            .node
            .fetch_blocks_non_empty(.., BlockListOrder::HighestToLowest)
            .into_iterator();

        let max_height = self.node.max_height();

        let earliest_date = last_day - chrono::Duration::days(7);
        let mut txns_by_days = HashMap::<NaiveDate, u64>::new();
        for block in blocks {
            let block = block?;

            let time = block.metadata().timestamp_unix_s;
            let block = block.into_block();
            let time = time.unwrap_or_else(|| {
                NodeShared::estimate_block_time(block.content.header.height, max_height)
            });

            let date = chrono::DateTime::<Utc>::from_timestamp(time as i64, 0)
                .unwrap()
                .date_naive();
            if date < earliest_date {
                break;
            }

            *txns_by_days.entry(date).or_insert(0) += block.content.state.txns.len() as u64;
        }

        let mut last_7_days = self.last_7_days.write();
        last_7_days.clear();
        for i in (0..7).rev() {
            let date = last_day - chrono::Duration::days(i);
            let txns = txns_by_days.get(&date).copied().unwrap_or(0);
            last_7_days.push((date, txns));
        }

        Ok(())
    }

    pub fn count_today_txns(&self) -> u64 {
        self.today.count()
    }

    pub async fn worker(self: Arc<Self>) -> Result<(), tokio::task::JoinError> {
        tokio::spawn(async move {
            loop {
                let today = Utc::now().date_naive();

                match self.refresh_last_7_days(today - chrono::Duration::days(1)) {
                    Ok(()) => {}
                    Err(error) => {
                        tracing::error!(
                            ?error,
                            "Failed to refresh transaction stats for the last 7 days"
                        );
                    }
                }

                self.ready.store(true, std::sync::atomic::Ordering::Relaxed);

                let tomorrow = today + chrono::Duration::days(1);
                let tomorrow_utc = chrono::DateTime::<Utc>::from_naive_utc_and_offset(
                    tomorrow.and_hms_opt(0, 0, 0).unwrap(),
                    Utc,
                );

                let sleep_duration_until_tomorrow = tomorrow_utc
                    .timestamp()
                    .saturating_sub(Utc::now().timestamp())
                    + 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    sleep_duration_until_tomorrow as u64,
                ))
                .await;
            }
        })
        .await
    }

    pub async fn today_stats_worker(self: Arc<Self>) -> Result<(), tokio::task::JoinError> {
        tokio::spawn(async move { self.today.clone().run().await }).await
    }
}
