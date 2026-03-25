use contextful::ResultContextExt;
use serde_json::json;
use web3::types::{BlockId, BlockNumber, H256};

use crate::{
    cli::BlockArgs,
    error::{Error, Result},
    output::{CommandOutput, with_loading},
    runtime::BeamApp,
};

pub async fn run(app: &BeamApp, args: BlockArgs) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let selector = args.block.unwrap_or_else(|| "latest".to_string());
    let block_id = parse_block_id(&selector)?;
    let block = with_loading(
        app.output_mode,
        format!("Fetching block {selector}..."),
        async {
            client
                .block(block_id)
                .await
                .context("fetch beam block")?
                .ok_or_else(|| Error::BlockNotFound {
                    block: selector.clone(),
                })
        },
    )
    .await?;
    let block_hash = block.hash.map(|value| format!("{value:#x}"));
    let block_number = block.number.map(|value| value.as_u64());
    let json_block = serde_json::to_value(&block).context("serialize beam block output")?;

    CommandOutput::new(
        render_block_default(&chain.entry.key, &selector, &block),
        json!({
            "block": json_block,
            "chain": chain.entry.key,
            "selector": selector,
        }),
    )
    .compact(
        block_hash
            .clone()
            .or_else(|| block_number.map(|value| value.to_string()))
            .unwrap_or_else(|| "unknown".to_string()),
    )
    .markdown(render_block_markdown(&chain.entry.key, &selector, &block))
    .print(app.output_mode)
}

pub(crate) fn parse_block_id(value: &str) -> Result<BlockId> {
    let value = value.trim();
    let block = match value {
        "latest" => BlockId::Number(BlockNumber::Latest),
        "earliest" => BlockId::Number(BlockNumber::Earliest),
        "pending" => BlockId::Number(BlockNumber::Pending),
        "safe" => BlockId::Number(BlockNumber::Safe),
        "finalized" | "finalised" => BlockId::Number(BlockNumber::Finalized),
        value if value.starts_with("0x") && value.len() == 66 => {
            BlockId::Hash(parse_hash(value).map_err(|_| Error::InvalidBlockSelector {
                value: value.to_string(),
            })?)
        }
        value if value.starts_with("0x") => {
            let block_number =
                u64::from_str_radix(value.trim_start_matches("0x"), 16).map_err(|_| {
                    Error::InvalidBlockSelector {
                        value: value.to_string(),
                    }
                })?;
            BlockId::Number(BlockNumber::Number(block_number.into()))
        }
        value => {
            let block_number = value
                .parse::<u64>()
                .map_err(|_| Error::InvalidBlockSelector {
                    value: value.to_string(),
                })?;
            BlockId::Number(BlockNumber::Number(block_number.into()))
        }
    };

    Ok(block)
}

fn parse_hash(value: &str) -> std::result::Result<H256, ()> {
    value.parse::<H256>().map_err(|_| ())
}

fn render_block_default(chain: &str, selector: &str, block: &web3::types::Block<H256>) -> String {
    let number = block
        .number
        .map_or_else(|| "unknown".to_string(), |value| value.as_u64().to_string());
    let hash = block
        .hash
        .map_or_else(|| "unknown".to_string(), |value| format!("{value:#x}"));
    let base_fee = block
        .base_fee_per_gas
        .map_or_else(|| "unknown".to_string(), |value| value.to_string());
    let miner = format!("{:#x}", block.author);
    let size = block
        .size
        .map_or_else(|| "unknown".to_string(), |value| value.to_string());

    format!(
        "Chain: {chain}\nSelector: {selector}\nNumber: {}\nHash: {}\nParent: {:#x}\nTimestamp: {}\nTransactions: {}\nGas used: {}\nGas limit: {}\nBase fee: {}\nMiner: {}\nSize: {}",
        number,
        hash,
        block.parent_hash,
        block.timestamp,
        block.transactions.len(),
        block.gas_used,
        block.gas_limit,
        base_fee,
        miner,
        size,
    )
}

fn render_block_markdown(chain: &str, selector: &str, block: &web3::types::Block<H256>) -> String {
    let number = block
        .number
        .map_or_else(|| "unknown".to_string(), |value| value.as_u64().to_string());
    let hash = block
        .hash
        .map_or_else(|| "unknown".to_string(), |value| format!("{value:#x}"));
    let base_fee = block
        .base_fee_per_gas
        .map_or_else(|| "unknown".to_string(), |value| value.to_string());
    let miner = format!("{:#x}", block.author);
    let size = block
        .size
        .map_or_else(|| "unknown".to_string(), |value| value.to_string());

    format!(
        "- Chain: `{chain}`\n- Selector: `{selector}`\n- Number: `{}`\n- Hash: `{}`\n- Parent: `{:#x}`\n- Timestamp: `{}`\n- Transactions: `{}`\n- Gas used: `{}`\n- Gas limit: `{}`\n- Base fee: `{}`\n- Miner: `{}`\n- Size: `{}`",
        number,
        hash,
        block.parent_hash,
        block.timestamp,
        block.transactions.len(),
        block.gas_used,
        block.gas_limit,
        base_fee,
        miner,
        size,
    )
}
