// lint-long-file-override allow-max-lines=300
use std::time::Duration;

use contextful::ResultContextExt;
use payy_evm_client::{Prepared, SubmittedOperationResult};
use serde::Serialize;
use serde_json::json;
use web3::ethabi::StateMutability;

use crate::{
    abi::parse_function,
    error::{Error, Result},
    evm::{call_function, format_units, parse_units},
    output::{
        CommandOutput, confirmed_transaction_message, dropped_transaction_message,
        pending_transaction_message, with_loading_handle,
    },
    privacy::{
        PrivacyContext, address_to_bytes, bytes_to_address, hex32,
        state::{PendingPrivacyOperation, PrivacyState, load_privacy_state},
    },
    runtime::{BeamApp, ResolvedToken},
    transaction::{TransactionExecution, loading_message, wait_for_completion},
};

use super::common::{field_to_address, resolve_token, save_checkpoint, state_key};

pub(super) async fn submit_and_record<T>(
    app: &BeamApp,
    ctx: &PrivacyContext,
    token: &ResolvedToken,
    operation: &str,
    submitted: SubmittedOperationResult<T>,
) -> Result<()>
where
    T: Serialize,
{
    let store = load_privacy_state(&app.paths.root).await?;
    let mut state = store.get().await;
    let key = state_key(ctx);
    let tx_hash = hex32(&submitted.source_tx_hash);
    state
        .entry_mut(&key)?
        .pending
        .push(PendingPrivacyOperation {
            operation: operation.to_string(),
            token: Some(format!("{:#x}", token.address)),
            tx_hash: tx_hash.clone(),
        });
    store
        .set(state.clone())
        .await
        .context("persist beam submitted privacy state")?;

    let client = ctx.adapter.client();
    let action = format!("private {operation}");
    let wait_tx_hash = tx_hash.clone();
    let execution = with_loading_handle(
        app.output_mode,
        format!("Waiting for private {operation} confirmation..."),
        |loading| async move {
            wait_for_completion(
                &client,
                wait_tx_hash,
                move |update| loading.set_message(loading_message(&action, &update)),
                tokio::signal::ctrl_c(),
                Duration::from_millis(750),
                Duration::from_secs(60),
            )
            .await
        },
    )
    .await?;

    match execution {
        TransactionExecution::Confirmed(outcome) => {
            refresh_checkpoint(ctx, token, &mut state).await?;
            state
                .entry_mut(&key)?
                .pending
                .retain(|pending| pending.tx_hash != tx_hash);
            store
                .set(state)
                .await
                .context("persist beam confirmed privacy state")?;
            render_write_output(
                ctx,
                token,
                operation,
                &tx_hash,
                outcome.block_number,
                outcome.status,
                &submitted.payload,
            )
            .print(app.output_mode)
        }
        TransactionExecution::Pending(pending) => render_pending_output(
            ctx,
            token,
            operation,
            &pending.tx_hash,
            pending.block_number,
            &submitted.payload,
        )
        .print(app.output_mode),
        TransactionExecution::Dropped(dropped) => render_dropped_output(
            ctx,
            token,
            operation,
            &dropped.tx_hash,
            dropped.block_number,
            &submitted.payload,
        )
        .print(app.output_mode),
    }
}

pub(super) async fn ensure_allowance(
    ctx: &PrivacyContext,
    token: &ResolvedToken,
    amount: contracts::U256,
) -> Result<()> {
    let function = parse_function(
        "allowance(address,address):(uint256)",
        StateMutability::View,
    )?;
    let bridge = ctx.profile.bridge_address()?;
    let outcome = call_function(
        &ctx.adapter.client(),
        Some(ctx.evm_address),
        token.address,
        &function,
        &[format!("{:#x}", ctx.evm_address), format!("{bridge:#x}")],
    )
    .await?;
    let allowance = outcome
        .decoded
        .as_ref()
        .and_then(|value| value.get(0))
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<contracts::U256>().ok())
        .unwrap_or_default();
    if allowance < amount {
        return Err(Error::PrivacyApprovalRequired {
            amount: format_units(amount, token.decimals.unwrap_or(18)),
            spender: format!("{bridge:#x}"),
            token: token.label.clone(),
        });
    }
    Ok(())
}

pub(super) fn parse_token_amount(token: &ResolvedToken, amount: &str) -> Result<contracts::U256> {
    parse_units(amount, usize::from(token.decimals.unwrap_or(18)))
}

pub(super) async fn token_from_prepared<T>(
    app: &BeamApp,
    ctx: &PrivacyContext,
    prepared: &Prepared<T>,
) -> Result<ResolvedToken> {
    token_from_bytes(app, ctx, prepared.prepared_call().state_preview.token).await
}

pub(super) async fn token_from_element(
    app: &BeamApp,
    ctx: &PrivacyContext,
    token: element::Element,
) -> Result<ResolvedToken> {
    token_from_bytes(app, ctx, field_to_address(token)).await
}

async fn refresh_checkpoint(
    ctx: &PrivacyContext,
    token: &ResolvedToken,
    state: &mut PrivacyState,
) -> Result<()> {
    let key = state_key(ctx);
    let checkpoint = ctx
        .client
        .privacy()
        .notes()
        .get(payy_evm_client::OwnedNoteGetParams {
            privacy_account: ctx.account.clone(),
            token: address_to_bytes(token.address),
        })
        .await
        .context("refresh beam privacy checkpoint")?;
    save_checkpoint(state, &key, token, checkpoint)
}

async fn token_from_bytes(
    app: &BeamApp,
    ctx: &PrivacyContext,
    token: [u8; 20],
) -> Result<ResolvedToken> {
    resolve_token(app, ctx, &format!("{:#x}", bytes_to_address(token))).await
}

fn render_write_output<T: Serialize>(
    ctx: &PrivacyContext,
    token: &ResolvedToken,
    operation: &str,
    tx_hash: &str,
    block_number: Option<u64>,
    status: Option<u64>,
    payload: &T,
) -> CommandOutput {
    CommandOutput::new(
        confirmed_transaction_message(
            format!("Confirmed private {operation}"),
            tx_hash,
            block_number,
        ),
        json!({
            "block_number": block_number,
            "chain": ctx.chain.entry.key,
            "operation": operation,
            "payload": serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
            "private_address": ctx.privacy_address_hex(),
            "state": "confirmed",
            "status": status,
            "token": token.label,
            "token_address": format!("{:#x}", token.address),
            "tx_hash": tx_hash,
        }),
    )
    .compact(tx_hash.to_string())
}

fn render_pending_output<T: Serialize>(
    ctx: &PrivacyContext,
    token: &ResolvedToken,
    operation: &str,
    tx_hash: &str,
    block_number: Option<u64>,
    payload: &T,
) -> CommandOutput {
    CommandOutput::new(
        pending_transaction_message(
            format!("Submitted private {operation} and stopped waiting for confirmation"),
            tx_hash,
            block_number,
        ),
        json!({
            "block_number": block_number,
            "chain": ctx.chain.entry.key,
            "operation": operation,
            "payload": serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
            "private_address": ctx.privacy_address_hex(),
            "state": "pending",
            "status": null,
            "token": token.label,
            "token_address": format!("{:#x}", token.address),
            "tx_hash": tx_hash,
        }),
    )
    .compact(tx_hash.to_string())
}

fn render_dropped_output<T: Serialize>(
    ctx: &PrivacyContext,
    token: &ResolvedToken,
    operation: &str,
    tx_hash: &str,
    block_number: Option<u64>,
    payload: &T,
) -> CommandOutput {
    CommandOutput::new(
        dropped_transaction_message(
            format!(
                "Submitted private {operation}, but the node no longer reports the transaction"
            ),
            tx_hash,
            block_number,
        ),
        json!({
            "block_number": block_number,
            "chain": ctx.chain.entry.key,
            "operation": operation,
            "payload": serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
            "private_address": ctx.privacy_address_hex(),
            "state": "dropped",
            "status": null,
            "token": token.label,
            "token_address": format!("{:#x}", token.address),
            "tx_hash": tx_hash,
        }),
    )
    .compact(tx_hash.to_string())
}
