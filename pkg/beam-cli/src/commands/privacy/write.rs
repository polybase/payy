// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use payy_evm_client::{
    BurnParams, ClaimLink, DirectSendParams, EvmAccount, IncomingTransfer, MintParams,
};

use crate::{
    cli::{PrivacyBurnArgs, PrivacySendArgs, PrivacyTokenAmountArgs},
    commands::privacy::{
        common::{resolve_token, state_key, u256_to_element},
        write_support::{
            ensure_allowance, parse_token_amount, submit_and_record, token_from_element,
            token_from_prepared,
        },
    },
    error::{Error, Result},
    output::with_loading,
    privacy::{
        address_to_bytes, load_privacy_context, parse_bytes32, parse_privacy_address,
        state::load_privacy_state,
    },
    runtime::BeamApp,
};

pub async fn mint(app: &BeamApp, args: PrivacyTokenAmountArgs) -> Result<()> {
    let ctx = load_privacy_context(app, "mint").await?;
    let token = resolve_token(app, &ctx, &args.token).await?;
    let amount = parse_token_amount(&token, &args.amount)?;
    ensure_allowance(&ctx, &token, amount).await?;
    let prepared = with_loading(app.output_mode, "Preparing private mint proof...", async {
        ctx.client
            .privacy()
            .mint(MintParams {
                privacy_account: ctx.account.clone(),
                evm_account: EvmAccount::Address(address_to_bytes(ctx.evm_address)),
                token: address_to_bytes(token.address),
                amount: u256_to_element(amount),
            })
            .prepare()
            .await
            .context("prepare beam private mint")
            .map_err(Into::into)
    })
    .await?;
    let submitted = with_loading(app.output_mode, "Submitting private mint...", async {
        prepared
            .submit()
            .await
            .context("submit beam private mint")
            .map_err(Into::into)
    })
    .await?;
    submit_and_record(app, &ctx, &token, "mint", submitted).await
}

pub async fn burn(app: &BeamApp, args: PrivacyBurnArgs) -> Result<()> {
    let ctx = load_privacy_context(app, "burn").await?;
    let token = resolve_token(app, &ctx, &args.token).await?;
    let recipient = app.resolve_wallet_or_address(&args.recipient).await?;
    let amount = parse_token_amount(&token, &args.amount)?;
    let prepared = with_loading(app.output_mode, "Preparing private burn proof...", async {
        ctx.client
            .privacy()
            .burn(BurnParams {
                privacy_account: ctx.account.clone(),
                token: address_to_bytes(token.address),
                amount: u256_to_element(amount),
                evm_recipient: address_to_bytes(recipient),
            })
            .prepare()
            .await
            .context("prepare beam private burn")
            .map_err(Into::into)
    })
    .await?;
    let submitted = with_loading(app.output_mode, "Submitting private burn...", async {
        prepared
            .submit()
            .await
            .context("submit beam private burn")
            .map_err(Into::into)
    })
    .await?;
    submit_and_record(app, &ctx, &token, "burn", submitted).await
}

pub async fn send(app: &BeamApp, args: PrivacySendArgs) -> Result<()> {
    if args.ephemeral {
        return super::ephemeral::send(app, args).await;
    }
    let (recipient, token_arg, amount_arg) = direct_send_args(&args)?;
    let ctx = load_privacy_context(app, "direct_send").await?;
    let token = resolve_token(app, &ctx, token_arg).await?;
    let amount = parse_token_amount(&token, amount_arg)?;
    let recipient = parse_privacy_address(recipient)?;
    let memo = args.memo.as_deref().map(parse_bytes32).transpose()?;
    let builder = ctx.client.privacy().send().to(DirectSendParams {
        privacy_account: ctx.account.clone(),
        token: address_to_bytes(token.address),
        amount: u256_to_element(amount),
        recipient,
        bridge_memo: memo,
    });

    if let Some(message) = args.claim_link_message.as_deref() {
        let prepared = with_loading(app.output_mode, "Preparing private send proof...", async {
            builder
                .link(Some(message))
                .await
                .context("prepare beam private send claim link")
                .map_err(Into::into)
        })
        .await?;
        let submitted = with_loading(app.output_mode, "Submitting private send...", async {
            prepared
                .submit()
                .await
                .context("submit beam private send")
                .map_err(Into::into)
        })
        .await?;
        return submit_and_record(app, &ctx, &token, "send", submitted).await;
    }

    let prepared = with_loading(app.output_mode, "Preparing private send proof...", async {
        builder
            .prepare()
            .await
            .context("prepare beam private send")
            .map_err(Into::into)
    })
    .await?;
    let submitted = with_loading(app.output_mode, "Submitting private send...", async {
        prepared
            .submit()
            .await
            .context("submit beam private send")
            .map_err(Into::into)
    })
    .await?;
    submit_and_record(app, &ctx, &token, "send", submitted).await
}

pub async fn claim(app: &BeamApp, source: &str) -> Result<()> {
    let ctx = load_privacy_context(app, "claim").await?;
    let store = load_privacy_state(&app.paths.root).await?;
    let state = store.get().await;
    let key = state_key(&ctx);
    let prepared = if source.trim_start().starts_with('{') {
        let transfer = serde_json::from_str::<IncomingTransfer>(source)
            .context("parse beam ephemeral incoming transfer")?;
        let token = token_from_element(app, &ctx, transfer.note.token).await?;
        let prepared = with_loading(app.output_mode, "Preparing private claim proof...", async {
            ctx.client
                .privacy()
                .claim()
                .account(ctx.account.clone())
                .ephemeral(transfer)
                .prepare()
                .await
                .context("prepare beam ephemeral claim")
                .map_err(Into::into)
        })
        .await?;
        let submitted = with_loading(app.output_mode, "Submitting private claim...", async {
            prepared
                .submit()
                .await
                .context("submit beam ephemeral claim")
                .map_err(Into::into)
        })
        .await?;
        return submit_and_record(app, &ctx, &token, "claim", submitted).await;
    } else if source.starts_with("payy:") {
        let parsed = ctx
            .client
            .privacy()
            .links()
            .parse(source)
            .context("parse beam claim link")?;
        let token_field = parsed
            .incoming_transfer
            .as_ref()
            .map(|transfer| transfer.note.token)
            .or_else(|| parsed.direct_note.as_ref().map(|note| note.note.token))
            .unwrap_or(element::Element::ZERO);
        let token = token_from_element(app, &ctx, token_field).await?;
        let prepared = with_loading(app.output_mode, "Preparing private claim proof...", async {
            ctx.client
                .privacy()
                .claim()
                .account(ctx.account.clone())
                .link(ClaimLink {
                    value: source.to_string(),
                })
                .prepare()
                .await
                .context("prepare beam claim link")
                .map_err(Into::into)
        })
        .await?;
        let submitted = with_loading(app.output_mode, "Submitting private claim...", async {
            prepared
                .submit()
                .await
                .context("submit beam claim link")
                .map_err(Into::into)
        })
        .await?;
        return submit_and_record(app, &ctx, &token, "claim", submitted).await;
    } else {
        let entry = state
            .entry(&key)?
            .ok_or_else(|| Error::PrivacyStateNotFound {
                id: source.to_string(),
            })?;
        let note = entry.incoming_note(source)?;
        with_loading(app.output_mode, "Preparing private claim proof...", async {
            ctx.client
                .privacy()
                .claim()
                .account(ctx.account.clone())
                .note(note)
                .prepare()
                .await
                .context("prepare beam incoming claim")
                .map_err(Into::into)
        })
        .await?
    };
    let token = token_from_prepared(app, &ctx, &prepared).await?;
    let submitted = with_loading(app.output_mode, "Submitting private claim...", async {
        prepared
            .submit()
            .await
            .context("submit beam private claim")
            .map_err(Into::into)
    })
    .await?;
    submit_and_record(app, &ctx, &token, "claim", submitted).await
}

fn direct_send_args(args: &PrivacySendArgs) -> Result<(&str, &str, &str)> {
    match args.args.as_slice() {
        [recipient, token, amount] => Ok((recipient, token, amount)),
        values => Err(Error::InvalidArgumentCount {
            expected: 3,
            got: values.len(),
        }),
    }
}
