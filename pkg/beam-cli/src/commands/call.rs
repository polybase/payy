use contracts::U256;
use serde_json::json;
use web3::ethabi::{Function, ParamType, StateMutability};

use crate::{
    abi::parse_function,
    cli::{CallArgs, SendArgs},
    commands::signing::prompt_active_signer,
    error::Result,
    evm::{FunctionCall, call_function, parse_units, send_function},
    output::{
        CommandOutput, confirmed_transaction_message, dropped_transaction_message,
        pending_transaction_message, with_loading, with_loading_handle,
    },
    runtime::{BeamApp, parse_address},
    transaction::{TransactionExecution, loading_message},
};

pub async fn run_read(app: &BeamApp, args: CallArgs) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let contract = parse_address(&args.contract)?;
    let function = parse_function(&args.function_sig, StateMutability::View)?;
    let call_args = resolve_address_args(app, &function, &args.args).await?;
    let from = app.active_optional_address().await?;
    let outcome = with_loading(
        app.output_mode,
        format!("Calling {contract:#x}..."),
        async { call_function(&client, from, contract, &function, &call_args).await },
    )
    .await?;
    let default = match &outcome.decoded {
        Some(decoded) => format!("Raw: {}\nDecoded: {decoded}", outcome.raw),
        None => outcome.raw.clone(),
    };

    CommandOutput::new(
        default,
        json!({
            "chain": chain.entry.key,
            "contract": format!("{contract:#x}"),
            "decoded": outcome.decoded,
            "raw": outcome.raw,
            "signature": args.function_sig,
        }),
    )
    .compact(outcome.raw)
    .print(app.output_mode)
}

pub async fn run_write(app: &BeamApp, args: SendArgs) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let chain_key = chain.entry.key.clone();
    let native_symbol = chain.entry.native_symbol.clone();
    let value_display = args.value.clone().unwrap_or_else(|| "0".to_string());
    let value = parse_transaction_value(args.value.as_deref())?;
    let contract = parse_address(&args.call.contract)?;
    let function = parse_function(&args.call.function_sig, StateMutability::NonPayable)?;
    let call_args = resolve_address_args(app, &function, &args.call.args).await?;
    let signer = prompt_active_signer(app).await?;
    let action = if value.is_zero() {
        format!("transaction to {contract:#x}")
    } else {
        format!("transaction to {contract:#x} with {value_display} {native_symbol}")
    };
    let execution = with_loading_handle(
        app.output_mode,
        format!("Sending {action} and waiting for confirmation..."),
        |loading| async move {
            send_function(
                &client,
                &signer,
                FunctionCall {
                    args: &call_args,
                    contract,
                    function: &function,
                    value,
                },
                move |update| loading.set_message(loading_message(&action, &update)),
                tokio::signal::ctrl_c(),
            )
            .await
        },
    )
    .await?;

    match execution {
        TransactionExecution::Confirmed(outcome) => {
            let tx_hash = outcome.tx_hash.clone();
            let block_number = outcome.block_number;
            let summary = if value.is_zero() {
                format!("Confirmed transaction to {contract:#x}")
            } else {
                format!(
                    "Confirmed transaction to {contract:#x} with {value_display} {native_symbol}"
                )
            };

            CommandOutput::new(
                confirmed_transaction_message(summary, &tx_hash, block_number),
                json!({
                    "block_number": block_number,
                    "chain": chain_key,
                    "contract": format!("{contract:#x}"),
                    "native_symbol": native_symbol,
                    "signature": args.call.function_sig,
                    "state": "confirmed",
                    "status": outcome.status,
                    "tx_hash": tx_hash,
                    "value": value_display,
                }),
            )
            .compact(outcome.tx_hash.clone())
            .print(app.output_mode)
        }
        TransactionExecution::Pending(pending) => {
            let tx_hash = pending.tx_hash.clone();
            let summary = if value.is_zero() {
                format!(
                    "Submitted transaction to {contract:#x} and stopped waiting for confirmation"
                )
            } else {
                format!(
                    "Submitted transaction to {contract:#x} with {value_display} {native_symbol} and stopped waiting for confirmation"
                )
            };

            CommandOutput::new(
                pending_transaction_message(summary, &tx_hash, pending.block_number),
                json!({
                    "block_number": pending.block_number,
                    "chain": chain_key,
                    "contract": format!("{contract:#x}"),
                    "native_symbol": native_symbol,
                    "signature": args.call.function_sig,
                    "state": "pending",
                    "status": null,
                    "tx_hash": tx_hash,
                    "value": value_display,
                }),
            )
            .compact(tx_hash)
            .print(app.output_mode)
        }
        TransactionExecution::Dropped(dropped) => {
            let tx_hash = dropped.tx_hash.clone();
            let summary = if value.is_zero() {
                format!(
                    "Submitted transaction to {contract:#x}, but the node no longer reports the transaction"
                )
            } else {
                format!(
                    "Submitted transaction to {contract:#x} with {value_display} {native_symbol}, but the node no longer reports the transaction"
                )
            };

            CommandOutput::new(
                dropped_transaction_message(summary, &tx_hash, dropped.block_number),
                json!({
                    "block_number": dropped.block_number,
                    "chain": chain_key,
                    "contract": format!("{contract:#x}"),
                    "native_symbol": native_symbol,
                    "signature": args.call.function_sig,
                    "state": "dropped",
                    "status": null,
                    "tx_hash": tx_hash,
                    "value": value_display,
                }),
            )
            .compact(dropped.tx_hash)
            .print(app.output_mode)
        }
    }
}

pub(crate) async fn resolve_address_args(
    app: &BeamApp,
    function: &Function,
    args: &[String],
) -> Result<Vec<String>> {
    if function.inputs.len() != args.len() {
        return Ok(args.to_vec());
    }

    let mut resolved = Vec::with_capacity(args.len());
    for (param, arg) in function.inputs.iter().zip(args) {
        if matches!(param.kind, ParamType::Address) {
            resolved.push(format!("{:#x}", app.resolve_wallet_or_address(arg).await?));
        } else {
            resolved.push(arg.clone());
        }
    }

    Ok(resolved)
}

pub(crate) fn parse_transaction_value(value: Option<&str>) -> Result<U256> {
    value.map_or(Ok(U256::zero()), |value| parse_units(value, 18))
}
