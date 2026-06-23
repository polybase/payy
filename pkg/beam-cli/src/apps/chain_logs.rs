use contextful::ResultContextExt;
use contracts::Client;
use serde_json::{Value, json};
use web3::types::{BlockNumber, FilterBuilder, H256, Log};

use crate::apps::{
    Error, Result,
    host::{ChainReadRequest, parse_host_address},
};

const MAX_LOG_BLOCK_RANGE: u64 = 50_000;
const MAX_LOG_TOPICS: usize = 4;
const MAX_LOG_TOPIC_VALUES: usize = 16;
const MAX_LOG_RESPONSE_BYTES: usize = 1024 * 1024;

pub async fn read(client: &Client, request: &ChainReadRequest) -> Result<Value> {
    let target = parse_host_address(
        "target",
        request
            .target
            .as_deref()
            .ok_or_else(|| Error::InvalidHostRequest {
                reason: "chain read missing target".to_string(),
            })?,
    )?;
    let latest = client
        .block_number()
        .await
        .context("fetch beam app latest block")?
        .as_u64();
    let to_block = request.to_block.unwrap_or(latest).min(latest);
    let from_block = request
        .from_block
        .unwrap_or_else(|| to_block.saturating_sub(MAX_LOG_BLOCK_RANGE));
    if from_block > to_block {
        return Err(Error::InvalidHostRequest {
            reason: format!("invalid log block range {from_block}..{to_block}"),
        });
    }
    if to_block.saturating_sub(from_block) > MAX_LOG_BLOCK_RANGE {
        return Err(Error::InvalidHostRequest {
            reason: format!("log block range exceeds {MAX_LOG_BLOCK_RANGE} blocks"),
        });
    }

    let topics = parse_topics(&request.topics)?;
    let filter = FilterBuilder::default()
        .address(vec![target])
        .from_block(BlockNumber::Number(from_block.into()))
        .to_block(BlockNumber::Number(to_block.into()))
        .topics(
            topics.first().cloned().unwrap_or(None),
            topics.get(1).cloned().unwrap_or(None),
            topics.get(2).cloned().unwrap_or(None),
            topics.get(3).cloned().unwrap_or(None),
        )
        .build();
    let logs = client.logs(filter).await.context("fetch beam app logs")?;
    let value = json!({
        "from_block": from_block,
        "logs": logs_json(&logs),
        "target": format!("{target:#x}"),
        "to_block": to_block,
    });
    let bytes = serde_json::to_vec(&value).context("measure beam app log response")?;
    if bytes.len() > MAX_LOG_RESPONSE_BYTES {
        return Err(Error::InvalidHostRequest {
            reason: format!("log response exceeds {MAX_LOG_RESPONSE_BYTES} bytes"),
        });
    }

    Ok(value)
}

fn parse_topics(topics: &[Option<Vec<String>>]) -> Result<Vec<Option<Vec<H256>>>> {
    if topics.len() > MAX_LOG_TOPICS {
        return Err(Error::InvalidHostRequest {
            reason: format!("log query supports at most {MAX_LOG_TOPICS} topic positions"),
        });
    }
    let mut out = Vec::new();
    for topic in topics {
        let Some(values) = topic else {
            out.push(None);
            continue;
        };
        if values.len() > MAX_LOG_TOPIC_VALUES {
            return Err(Error::InvalidHostRequest {
                reason: format!(
                    "log query supports at most {MAX_LOG_TOPIC_VALUES} values per topic"
                ),
            });
        }
        let mut parsed = Vec::new();
        for value in values {
            parsed.push(
                value
                    .parse::<H256>()
                    .map_err(|_| Error::InvalidHostRequest {
                        reason: format!("invalid log topic {value}"),
                    })?,
            );
        }
        out.push(Some(parsed));
    }

    Ok(out)
}

fn logs_json(logs: &[Log]) -> Vec<Value> {
    logs.iter()
        .map(|log| {
            json!({
                "address": format!("{:#x}", log.address),
                "block_hash": log.block_hash.map(|value| format!("{value:#x}")),
                "block_number": log.block_number.map(|value| value.as_u64()),
                "data": format!("0x{}", hex::encode(&log.data.0)),
                "log_index": log.log_index.map(|value| value.to_string()),
                "topics": log
                    .topics
                    .iter()
                    .map(|topic| format!("{topic:#x}"))
                    .collect::<Vec<_>>(),
                "transaction_hash": log.transaction_hash.map(|value| format!("{value:#x}")),
                "transaction_index": log.transaction_index.map(|value| value.to_string()),
            })
        })
        .collect()
}
