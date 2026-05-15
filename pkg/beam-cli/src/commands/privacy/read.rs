// lint-long-file-override allow-max-lines=300
use std::time::{SystemTime, UNIX_EPOCH};

use contextful::ResultContextExt;
use payy_evm_client::{IncomingListParams, IncomingNote, OwnedNoteGetParams, PayyEvmReadClient};
use serde_json::json;
use tokio::time::{Duration, sleep};

use crate::{
    cli::{PrivacyBalanceArgs, PrivacyIncomingListArgs, PrivacyIncomingWatchArgs},
    error::{Error, Result},
    output::{CommandOutput, with_loading},
    privacy::{
        PrivacyContext, address_to_bytes, load_privacy_context,
        state::{PrivacyState, PrivacyStateKey, load_privacy_state},
    },
    runtime::{BeamApp, ResolvedToken},
};

use super::{
    common::{element_to_u256, resolve_token, save_checkpoint, state_key},
    render::{PrivateBalanceRow, render_balances, render_incoming},
};

pub async fn address(app: &BeamApp) -> Result<()> {
    let ctx = load_privacy_context(app, "address").await?;
    let private_address = ctx.privacy_address_hex();
    let evm_address = format!("{:#x}", ctx.evm_address);
    CommandOutput::new(
        format!("Private address: {private_address}\nEVM wallet: {evm_address}"),
        json!({
            "chain": ctx.chain.entry.key,
            "evm_address": evm_address,
            "private_address": private_address,
        }),
    )
    .compact(private_address)
    .print(app.output_mode)
}

pub async fn balance(app: &BeamApp, args: PrivacyBalanceArgs) -> Result<()> {
    let ctx = load_privacy_context(app, "balance").await?;
    let tokens = resolve_balance_tokens(app, &ctx, args.token).await?;
    let store = load_privacy_state(&app.paths.root).await?;
    let mut state = store.get().await;
    let key = state_key(&ctx);
    seed_checkpoints(&ctx, &state, &key, &tokens)?;
    let rows = with_loading(app.output_mode, "Fetching private balances...", async {
        let mut rows = Vec::new();
        for token in &tokens {
            let params = OwnedNoteGetParams {
                privacy_account: ctx.account.clone(),
                token: address_to_bytes(token.address),
            };
            let balance = ctx
                .client
                .privacy()
                .balances()
                .get(params)
                .await
                .context("fetch beam private balance")?;
            let spendable = balance
                .balance
                .as_ref()
                .map_or(element::Element::ZERO, |balance| balance.spendable);
            rows.push(PrivateBalanceRow {
                decimals: token.decimals.unwrap_or(18),
                label: token.label.clone(),
                token_address: format!("{:#x}", token.address),
                value: element_to_u256(spendable),
            });
            save_checkpoint(&mut state, &key, token, balance.owned_note_state)?;
        }
        Ok(rows)
    })
    .await?;
    store
        .set(state)
        .await
        .context("persist beam privacy state")?;
    render_balances(&ctx, &rows).print(app.output_mode)
}

pub async fn incoming_list(app: &BeamApp, args: PrivacyIncomingListArgs) -> Result<()> {
    let ctx = load_privacy_context(app, "incoming").await?;
    let store = load_privacy_state(&app.paths.root).await?;
    let mut state = store.get().await;
    let key = state_key(&ctx);
    let from_block = args.from_block.unwrap_or_else(|| {
        state
            .entry(&key)
            .ok()
            .flatten()
            .map_or(0, |entry| entry.incoming_next_block)
    });
    let notes = with_loading(
        app.output_mode,
        "Fetching incoming private transfers...",
        async {
            ctx.client
                .privacy()
                .incoming()
                .list(IncomingListParams {
                    privacy_account: ctx.account.clone(),
                    privacy_address_prefix: None,
                    from_block,
                    to_block: args.to_block,
                    include_spent: args.include_spent,
                    poll_interval_ms: None,
                })
                .await
                .context("list beam incoming private transfers")
                .map_err(Into::into)
        },
    )
    .await?;
    remember_incoming(&mut state, &key, &notes, args.to_block)?;
    store
        .set(state)
        .await
        .context("persist beam privacy incoming state")?;
    render_incoming(&ctx, &notes).print(app.output_mode)
}

pub async fn incoming_watch(app: &BeamApp, args: PrivacyIncomingWatchArgs) -> Result<()> {
    let ctx = load_privacy_context(app, "incoming").await?;
    let store = load_privacy_state(&app.paths.root).await?;
    let mut state = store.get().await;
    let key = state_key(&ctx);
    let mut from_block = args.from_block.unwrap_or_else(|| {
        state
            .entry(&key)
            .ok()
            .flatten()
            .map_or(0, |entry| entry.incoming_next_block)
    });
    let poll = Duration::from_millis(args.poll_interval_ms.unwrap_or(3000));
    let mut discovered = Vec::new();

    loop {
        let head = ctx
            .adapter
            .get_block_number()
            .await
            .context("fetch beam privacy watch head")?;
        if from_block <= head {
            let notes = ctx
                .client
                .privacy()
                .incoming()
                .list(IncomingListParams {
                    privacy_account: ctx.account.clone(),
                    privacy_address_prefix: None,
                    from_block,
                    to_block: Some(head),
                    include_spent: args.include_spent,
                    poll_interval_ms: None,
                })
                .await
                .context("watch beam incoming private transfers")?;
            remember_incoming(&mut state, &key, &notes, Some(head))?;
            discovered.extend(notes);
            store
                .set(state.clone())
                .await
                .context("persist beam privacy watch state")?;
            from_block = head.saturating_add(1);
        }

        tokio::select! {
            _ = sleep(poll) => {}
            signal = tokio::signal::ctrl_c() => {
                signal.context("listen for beam privacy watch ctrl-c")?;
                return render_incoming(&ctx, &discovered).print(app.output_mode);
            }
        }
    }
}

pub async fn state_reset(app: &BeamApp) -> Result<()> {
    let ctx = load_privacy_context(app, "state").await?;
    let store = load_privacy_state(&app.paths.root).await?;
    let mut state = store.get().await;
    let key = state_key(&ctx);
    state.entries.remove(&key.id());
    store
        .set(state)
        .await
        .context("persist beam privacy state reset")?;
    CommandOutput::message("Reset privacy state for the active wallet and chain")
        .print(app.output_mode)
}

pub async fn state_repair(app: &BeamApp) -> Result<()> {
    let path = app.paths.root.join("privacy-state.json");
    if !path.exists() {
        return CommandOutput::message("No privacy state file to repair").print(app.output_mode);
    }

    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("read beam privacy state repair timestamp")?
        .as_secs();
    let backup = app
        .paths
        .root
        .join(format!("privacy-state.json.invalid-{suffix}"));
    tokio::fs::rename(&path, &backup)
        .await
        .context("move corrupted beam privacy state aside")?;
    CommandOutput::new(
        format!("Moved privacy state to {}", backup.display()),
        serde_json::json!({
            "backup": backup,
            "message": "Moved privacy state aside",
        }),
    )
    .print(app.output_mode)
}

fn seed_checkpoints(
    ctx: &PrivacyContext,
    state: &PrivacyState,
    key: &PrivacyStateKey,
    tokens: &[ResolvedToken],
) -> Result<()> {
    let Some(entry) = state.entry(key)? else {
        return Ok(());
    };
    for token in tokens {
        if let Some(checkpoint) = entry.checkpoint(&format!("{:#x}", token.address)) {
            ctx.client
                .privacy()
                .set_checkpoint(checkpoint)
                .context("seed beam privacy checkpoint")?;
        }
    }
    Ok(())
}

fn remember_incoming(
    state: &mut PrivacyState,
    key: &PrivacyStateKey,
    notes: &[IncomingNote],
    to_block: Option<u64>,
) -> Result<()> {
    let entry = state.entry_mut(key)?;
    for note in notes {
        entry.remember_incoming(note.clone());
    }
    if let Some(to_block) = to_block {
        entry.incoming_next_block = to_block.saturating_add(1);
    } else if let Some(last) = notes.last() {
        entry.incoming_next_block = last.source_position.block_number.saturating_add(1);
    }
    Ok(())
}

async fn resolve_balance_tokens(
    app: &BeamApp,
    ctx: &PrivacyContext,
    token: Option<String>,
) -> Result<Vec<ResolvedToken>> {
    if let Some(token) = token {
        return Ok(vec![resolve_token(app, ctx, &token).await?]);
    }
    let tracked = app.tracked_tokens_for_chain(&ctx.chain.entry.key).await;
    tracked
        .into_iter()
        .map(|token| {
            Ok(ResolvedToken {
                address: token.address.parse().map_err(|_| Error::InvalidAddress {
                    value: token.address.clone(),
                })?,
                decimals: Some(token.decimals),
                label: token.label,
            })
        })
        .collect()
}
