// lint-long-file-override allow-max-lines=300
use std::time::Duration;

use contextful::ResultContextExt;
use serde_json::json;

use crate::{
    commands::fetch::payment::{ExecutedPayment, PaymentAssetKind, PreparedPayment},
    error::{Error, Result},
    output::{with_loading, with_loading_handle},
    privacy::{
        PrivacyContext, address_to_bytes, hex32, load_privacy_context, parse_privacy_address,
        state::{PendingPrivacyOperation, PrivacyStateKey, load_privacy_state},
    },
    runtime::{BeamApp, ResolvedToken},
    transaction::{TransactionExecution, loading_message, wait_for_completion},
};

pub(super) async fn execute_private_payment(
    app: &BeamApp,
    payment: &PreparedPayment,
) -> Result<ExecutedPayment> {
    let private_recipient = payment
        .private_recipient
        .as_ref()
        .ok_or(Error::FetchInvalidPaymentResponse)?;
    let private_recipient = parse_privacy_address(private_recipient)?;
    if payment.chain.privacy.is_none() {
        return Err(Error::PrivacyNotConfigured {
            chain: payment.chain.key.clone(),
        });
    }

    let ctx = load_privacy_context(app, "private_payments").await?;
    if ctx.chain.entry.chain_id != payment.chain.chain_id {
        return Err(Error::FetchPaymentChainMismatch {
            challenge: payment.chain.summary(),
            selected: ctx.chain.entry.display_name.clone(),
        });
    }
    let PaymentAssetKind::Erc20(token_address) = payment.asset.kind else {
        return Err(Error::FetchInvalidPaymentResponse);
    };
    let token = app
        .token_for_chain(&format!("{token_address:#x}"), &ctx.chain.entry.key)
        .await?;
    let prepared = with_loading(
        app.output_mode,
        "Preparing private fetch payment...",
        async {
            ctx.client
                .privacy()
                .send()
                .to(payy_evm_client::DirectSendParams {
                    privacy_account: ctx.account.clone(),
                    token: address_to_bytes(token.address),
                    amount: u256_to_element(payment.amount),
                    recipient: private_recipient,
                    bridge_memo: None,
                })
                .prepare()
                .await
                .context("prepare beam fetch private payment")
                .map_err(Into::into)
        },
    )
    .await?;
    let submitted = with_loading(
        app.output_mode,
        "Submitting private fetch payment...",
        async {
            prepared
                .submit()
                .await
                .context("submit beam fetch private payment")
                .map_err(Into::into)
        },
    )
    .await?;
    persist_submitted(app, &ctx, &token, hex32(&submitted.source_tx_hash)).await?;

    let tx_hash = hex32(&submitted.source_tx_hash);
    let client = ctx.adapter.client();
    let wait_hash = tx_hash.clone();
    let execution = with_loading_handle(
        app.output_mode,
        "Waiting for private fetch payment confirmation...",
        |loading| async move {
            wait_for_completion(
                &client,
                wait_hash,
                move |update| {
                    loading.set_message(loading_message("private fetch payment", &update))
                },
                tokio::signal::ctrl_c(),
                Duration::from_millis(750),
                Duration::from_secs(60),
            )
            .await
        },
    )
    .await?;

    match execution {
        TransactionExecution::Confirmed(_) => {
            persist_confirmed(app, &ctx, &token, &tx_hash).await?;
            Ok(ExecutedPayment {
                accepted: payment.accepted.clone(),
                network: payment.network.clone(),
                proof: json!({
                    "amount": payment.amount.to_string(),
                    "asset": payment.asset_id,
                    "chainId": payment.chain.chain_id,
                    "from": format!("{:#x}", payment.payer),
                    "kind": "beam-private-transfer",
                    "network": payment.network,
                    "privateRecipient": payment.private_recipient,
                    "txHash": tx_hash,
                }),
                scheme: payment.scheme.clone(),
                source: Some(format!(
                    "did:pkh:eip155:{}:{:#x}",
                    payment.chain.chain_id, payment.payer
                )),
            })
        }
        TransactionExecution::Pending(pending) => Err(Error::FetchPaymentUnconfirmed {
            tx_hash: pending.tx_hash,
        }),
        TransactionExecution::Dropped(dropped) => Err(Error::FetchPaymentUnconfirmed {
            tx_hash: dropped.tx_hash,
        }),
    }
}

async fn persist_submitted(
    app: &BeamApp,
    ctx: &PrivacyContext,
    token: &ResolvedToken,
    tx_hash: String,
) -> Result<()> {
    let store = load_privacy_state(&app.paths.root).await?;
    let mut state = store.get().await;
    state
        .entry_mut(&state_key(ctx))?
        .pending
        .push(PendingPrivacyOperation {
            operation: "fetch-payment".to_string(),
            token: Some(format!("{:#x}", token.address)),
            tx_hash,
        });
    store
        .set(state)
        .await
        .context("persist beam private fetch payment state")?;
    Ok(())
}

async fn persist_confirmed(
    app: &BeamApp,
    ctx: &PrivacyContext,
    token: &ResolvedToken,
    tx_hash: &str,
) -> Result<()> {
    let store = load_privacy_state(&app.paths.root).await?;
    let mut state = store.get().await;
    let checkpoint = ctx
        .client
        .privacy()
        .notes()
        .get(payy_evm_client::OwnedNoteGetParams {
            privacy_account: ctx.account.clone(),
            token: address_to_bytes(token.address),
        })
        .await
        .context("refresh beam private fetch payment checkpoint")?;
    let entry = state.entry_mut(&state_key(ctx))?;
    entry.token_mut(&format!("{:#x}", token.address)).checkpoint = Some(checkpoint);
    entry.pending.retain(|pending| pending.tx_hash != tx_hash);
    store
        .set(state)
        .await
        .context("persist beam confirmed private fetch payment state")?;
    Ok(())
}

fn state_key(ctx: &PrivacyContext) -> PrivacyStateKey {
    PrivacyStateKey {
        bridge: ctx.profile.bridge.clone(),
        chain: ctx.chain.entry.key.clone(),
        chain_id: ctx.chain.entry.chain_id,
        privacy_address: ctx.privacy_address_hex(),
        standard: ctx.profile.standard.clone(),
        standard_version: ctx.profile.version,
        wallet_address: format!("{:#x}", ctx.evm_address),
    }
}

fn u256_to_element(value: contracts::U256) -> element::Element {
    let mut bytes = [0u8; 32];
    value.to_big_endian(&mut bytes);
    element::Element::from_be_bytes(bytes)
}
