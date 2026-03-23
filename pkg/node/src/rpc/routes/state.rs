use crate::{NodeShared, rpc::stats};
use std::sync::Arc;

pub struct State {
    pub node: Arc<NodeShared>,
    pub health_check_commit_interval_sec: u64,
    pub(crate) txn_stats: Arc<stats::TxnStats>,
}
