use serde_json::json;

use crate::{
    cli::TransferArgs,
    commands::signing::prompt_active_signer,
    error::Result,
    evm::{parse_units, send_native},
    output::{
        CommandOutput, confirmed_transaction_message, dropped_transaction_message,
        pending_transaction_message, with_loading_handle,
    },
    runtime::BeamApp,
    transaction::{TransactionExecution, loading_message},
};

pub async fn run(app: &BeamApp, args: TransferArgs) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let to = app.resolve_wallet_or_address(&args.to).await?;
    let amount = parse_units(&args.amount, 18)?;
    let signer = prompt_active_signer(app).await?;
    let action = format!(
        "transfer of {} {} to {to:#x}",
        args.amount, chain.entry.native_symbol
    );
    let execution = with_loading_handle(
        app.output_mode,
        format!("Sending {action} and waiting for confirmation..."),
        |loading| async move {
            send_native(
                &client,
                &signer,
                to,
                amount,
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

            CommandOutput::new(
                confirmed_transaction_message(
                    format!(
                        "Confirmed transfer of {} {} to {to:#x}",
                        args.amount, chain.entry.native_symbol
                    ),
                    &tx_hash,
                    block_number,
                ),
                json!({
                    "amount": args.amount,
                    "block_number": block_number,
                    "chain": chain.entry.key,
                    "native_symbol": chain.entry.native_symbol,
                    "state": "confirmed",
                    "status": outcome.status,
                    "to": format!("{to:#x}"),
                    "tx_hash": tx_hash,
                }),
            )
            .compact(outcome.tx_hash.clone())
            .markdown(format!(
                "- State: `confirmed`\n- Amount: `{}` `{}`\n- To: `{to:#x}`\n- Tx: `{}`\n- Block: `{}`",
                args.amount,
                chain.entry.native_symbol,
                outcome.tx_hash,
                block_number.map_or_else(|| "unknown".to_string(), |value| value.to_string()),
            ))
            .print(app.output_mode)
        }
        TransactionExecution::Pending(pending) => CommandOutput::new(
            pending_transaction_message(
                format!(
                    "Submitted transfer of {} {} to {to:#x} and stopped waiting for confirmation",
                    args.amount, chain.entry.native_symbol
                ),
                &pending.tx_hash,
                pending.block_number,
            ),
            json!({
                "amount": args.amount,
                "block_number": pending.block_number,
                "chain": chain.entry.key,
                "native_symbol": chain.entry.native_symbol,
                "state": "pending",
                "status": null,
                "to": format!("{to:#x}"),
                "tx_hash": pending.tx_hash.clone(),
            }),
        )
        .compact(pending.tx_hash.clone())
        .markdown(format!(
            "- State: `pending`\n- Amount: `{}` `{}`\n- To: `{to:#x}`\n- Tx: `{}`\n- Block: `{}`\n- Note: `Stopped waiting for confirmation`",
            args.amount,
            chain.entry.native_symbol,
            pending.tx_hash,
            pending
                .block_number
                .map_or_else(|| "pending".to_string(), |value| value.to_string()),
        ))
        .print(app.output_mode),
        TransactionExecution::Dropped(dropped) => CommandOutput::new(
            dropped_transaction_message(
                format!(
                    "Submitted transfer of {} {} to {to:#x}, but the node no longer reports the transaction",
                    args.amount, chain.entry.native_symbol
                ),
                &dropped.tx_hash,
                dropped.block_number,
            ),
            json!({
                "amount": args.amount,
                "block_number": dropped.block_number,
                "chain": chain.entry.key,
                "native_symbol": chain.entry.native_symbol,
                "state": "dropped",
                "status": null,
                "to": format!("{to:#x}"),
                "tx_hash": dropped.tx_hash.clone(),
            }),
        )
        .compact(dropped.tx_hash.clone())
        .markdown(format!(
            "- State: `dropped`\n- Amount: `{}` `{}`\n- To: `{to:#x}`\n- Tx: `{}`\n- Last seen block: `{}`\n- Note: `The RPC no longer reports the transaction`",
            args.amount,
            chain.entry.native_symbol,
            dropped.tx_hash,
            dropped
                .block_number
                .map_or_else(|| "pending".to_string(), |value| value.to_string()),
        ))
        .print(app.output_mode),
    }
}
