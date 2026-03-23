use actix_web::web;

use crate::State;
use rpc::error::HttpResult;

use super::error;

#[derive(Debug, serde::Serialize)]
struct TxnDayStats {
    date: chrono::NaiveDate,
    count: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct StatsResponse {
    last_7_days_txns: Vec<TxnDayStats>,
    today_txns: u64,
}

#[tracing::instrument(err, skip_all)]
pub async fn get_stats(state: web::Data<State>) -> HttpResult<web::Json<StatsResponse>> {
    if !state.txn_stats.ready() {
        return Err(error::Error::StatisticsNotReady)?;
    }

    let last_7_days_txns = state
        .txn_stats
        .last_7_days()
        .into_iter()
        .map(|(date, count)| TxnDayStats { date, count })
        .collect();

    let today_txns = state.txn_stats.count_today_txns();

    Ok(web::Json(StatsResponse {
        last_7_days_txns,
        today_txns,
    }))
}
