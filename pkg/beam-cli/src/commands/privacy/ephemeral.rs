use contextful::ResultContextExt;
use payy_evm_client::EphemeralSendParams;

use crate::{
    cli::PrivacySendArgs,
    commands::privacy::{
        common::{resolve_token, u256_to_element},
        write_support::{parse_token_amount, submit_and_record},
    },
    error::{Error, Result},
    output::with_loading,
    privacy::{address_to_bytes, load_privacy_context, parse_bytes32},
    runtime::BeamApp,
};

pub async fn send(app: &BeamApp, args: PrivacySendArgs) -> Result<()> {
    let (token_arg, amount_arg) = ephemeral_send_args(&args)?;
    let ctx = load_privacy_context(app, "ephemeral").await?;
    let token = resolve_token(app, &ctx, token_arg).await?;
    let amount = parse_token_amount(&token, amount_arg)?;
    let memo = args.memo.as_deref().map(parse_bytes32).transpose()?;
    let builder = ctx.client.privacy().send().ephemeral(EphemeralSendParams {
        privacy_account: ctx.account.clone(),
        token: address_to_bytes(token.address),
        amount: u256_to_element(amount),
        bridge_memo: memo,
    });

    if let Some(message) = args.claim_link_message.as_deref() {
        let prepared = with_loading(
            app.output_mode,
            "Preparing ephemeral private send...",
            async {
                builder
                    .link(Some(message))
                    .await
                    .context("prepare beam ephemeral claim link")
                    .map_err(Into::into)
            },
        )
        .await?;
        let submitted = with_loading(
            app.output_mode,
            "Submitting ephemeral private send...",
            async {
                prepared
                    .submit()
                    .await
                    .context("submit beam ephemeral send")
                    .map_err(Into::into)
            },
        )
        .await?;
        return submit_and_record(app, &ctx, &token, "ephemeral-send", submitted).await;
    }

    let prepared = with_loading(
        app.output_mode,
        "Preparing ephemeral private send...",
        async {
            builder
                .prepare()
                .await
                .context("prepare beam ephemeral send")
                .map_err(Into::into)
        },
    )
    .await?;
    let submitted = with_loading(
        app.output_mode,
        "Submitting ephemeral private send...",
        async {
            prepared
                .submit()
                .await
                .context("submit beam ephemeral send")
                .map_err(Into::into)
        },
    )
    .await?;
    submit_and_record(app, &ctx, &token, "ephemeral-send", submitted).await
}

fn ephemeral_send_args(args: &PrivacySendArgs) -> Result<(&str, &str)> {
    match args.args.as_slice() {
        [token, amount] => Ok((token, amount)),
        values => Err(Error::InvalidArgumentCount {
            expected: 2,
            got: values.len(),
        }),
    }
}
