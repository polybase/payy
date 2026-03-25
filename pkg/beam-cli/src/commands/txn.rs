use contextful::ResultContextExt;
use serde_json::json;
use web3::types::H256;

use crate::{
    cli::TxnArgs,
    error::{Error, Result},
    evm::format_units,
    output::{CommandOutput, with_loading},
    runtime::BeamApp,
};

pub async fn run(app: &BeamApp, args: TxnArgs) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let tx_hash = parse_tx_hash(&args.tx_hash)?;
    let (transaction, receipt) = with_loading(
        app.output_mode,
        format!("Fetching transaction {tx_hash:#x}..."),
        async {
            let transaction = client
                .transaction(tx_hash)
                .await
                .context("fetch beam transaction")?
                .ok_or_else(|| Error::TransactionNotFound {
                    tx_hash: format!("{tx_hash:#x}"),
                })?;
            let receipt = client
                .transaction_receipt(tx_hash)
                .await
                .context("fetch beam transaction receipt")?;
            Ok::<_, Error>((transaction, receipt))
        },
    )
    .await?;
    let state = transaction_state(&transaction, receipt.as_ref());
    let json_transaction =
        serde_json::to_value(&transaction).context("serialize beam transaction output")?;
    let json_receipt = receipt
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .context("serialize beam transaction receipt output")?;

    CommandOutput::new(
        render_transaction_default(
            &chain.entry.key,
            &chain.entry.native_symbol,
            &transaction,
            receipt.as_ref(),
            state,
        ),
        json!({
            "chain": chain.entry.key,
            "receipt": json_receipt,
            "state": state,
            "transaction": json_transaction,
            "tx_hash": format!("{tx_hash:#x}"),
        }),
    )
    .compact(format!("{tx_hash:#x}"))
    .markdown(render_transaction_markdown(
        &chain.entry.key,
        &chain.entry.native_symbol,
        &transaction,
        receipt.as_ref(),
        state,
    ))
    .print(app.output_mode)
}

pub(crate) fn parse_tx_hash(value: &str) -> Result<H256> {
    value
        .parse::<H256>()
        .map_err(|_| Error::InvalidTransactionHash {
            value: value.to_string(),
        })
}

fn render_transaction_default(
    chain: &str,
    native_symbol: &str,
    transaction: &web3::types::Transaction,
    receipt: Option<&web3::types::TransactionReceipt>,
    state: &str,
) -> String {
    let to = transaction.to.map_or_else(
        || "contract creation".to_string(),
        |value| format!("{value:#x}"),
    );
    let gas_price = transaction
        .gas_price
        .map_or_else(|| "unknown".to_string(), |value| value.to_string());
    let receipt_status = receipt
        .and_then(|value| value.status.map(|status| status.as_u64()))
        .map_or_else(|| "pending".to_string(), |value| value.to_string());
    let gas_used = receipt
        .and_then(|value| value.gas_used)
        .map_or_else(|| "pending".to_string(), |value| value.to_string());

    format!(
        "Chain: {chain}\nHash: {:#x}\nState: {state}\nFrom: {:#x}\nTo: {to}\nNonce: {}\nValue: {} {native_symbol} ({} wei)\nGas: {}\nGas price: {gas_price}\nBlock: {}\nIndex: {}\nReceipt status: {receipt_status}\nGas used: {gas_used}\nInput: 0x{}",
        transaction.hash,
        transaction.from.unwrap_or_default(),
        transaction.nonce,
        format_units(transaction.value, 18),
        transaction.value,
        transaction.gas,
        transaction
            .block_number
            .map_or_else(|| "pending".to_string(), |value| value.as_u64().to_string()),
        transaction
            .transaction_index
            .map_or_else(|| "pending".to_string(), |value| value.as_u64().to_string()),
        hex::encode(&transaction.input.0),
    )
}

fn render_transaction_markdown(
    chain: &str,
    native_symbol: &str,
    transaction: &web3::types::Transaction,
    receipt: Option<&web3::types::TransactionReceipt>,
    state: &str,
) -> String {
    let to = transaction.to.map_or_else(
        || "contract creation".to_string(),
        |value| format!("{value:#x}"),
    );
    let gas_price = transaction
        .gas_price
        .map_or_else(|| "unknown".to_string(), |value| value.to_string());
    let receipt_status = receipt
        .and_then(|value| value.status.map(|status| status.as_u64()))
        .map_or_else(|| "pending".to_string(), |value| value.to_string());
    let gas_used = receipt
        .and_then(|value| value.gas_used)
        .map_or_else(|| "pending".to_string(), |value| value.to_string());

    format!(
        "- Chain: `{chain}`\n- Hash: `{:#x}`\n- State: `{state}`\n- From: `{:#x}`\n- To: `{to}`\n- Nonce: `{}`\n- Value: `{}` `{native_symbol}` (`{}` wei)\n- Gas: `{}`\n- Gas price: `{gas_price}`\n- Block: `{}`\n- Index: `{}`\n- Receipt status: `{receipt_status}`\n- Gas used: `{gas_used}`\n- Input: `0x{}`",
        transaction.hash,
        transaction.from.unwrap_or_default(),
        transaction.nonce,
        format_units(transaction.value, 18),
        transaction.value,
        transaction.gas,
        transaction
            .block_number
            .map_or_else(|| "pending".to_string(), |value| value.as_u64().to_string()),
        transaction
            .transaction_index
            .map_or_else(|| "pending".to_string(), |value| value.as_u64().to_string()),
        hex::encode(&transaction.input.0),
    )
}

fn transaction_state(
    transaction: &web3::types::Transaction,
    receipt: Option<&web3::types::TransactionReceipt>,
) -> &'static str {
    match receipt.and_then(|value| value.status.map(|status| status.as_u64())) {
        Some(1) => "confirmed",
        Some(_) => "reverted",
        None if transaction.block_number.is_some() => "mined",
        None => "pending",
    }
}
