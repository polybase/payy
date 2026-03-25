use serde_json::json;

use crate::{
    cli::BalanceArgs,
    commands::{erc20, tokens},
    error::{Error, Result},
    evm::{erc20_balance, format_units, native_balance},
    human_output::sanitize_control_chars,
    output::{CommandOutput, balance_message, with_loading},
    runtime::BeamApp,
};

pub async fn run(app: &BeamApp, args: BalanceArgs) -> Result<()> {
    let Some(token_selector) = args.token else {
        return tokens::list_tokens(app).await;
    };
    let (chain, client) = app.active_chain_client().await?;
    let address = app.active_address().await?;
    if !tokens::is_native_selector(&token_selector, &chain.entry.native_symbol) {
        let token = app
            .token_for_chain(&token_selector, &chain.entry.key)
            .await?;
        let display_label = sanitize_control_chars(&token.label);
        let (label, decimals, balance) = with_loading(
            app.output_mode,
            format!("Fetching {display_label} balance for {address:#x}..."),
            async {
                let (label, decimals) = tokens::resolve_erc20_metadata(&client, &token).await?;
                let balance = erc20_balance(&client, token.address, address).await?;
                Ok::<_, Error>((label, decimals, balance))
            },
        )
        .await?;
        let formatted = format_units(balance, decimals);

        return erc20::render_balance_output(
            &chain.entry.key,
            &label,
            &format!("{:#x}", token.address),
            &format!("{address:#x}"),
            &formatted,
            decimals,
            &balance.to_string(),
        )
        .print(app.output_mode);
    }

    let balance = with_loading(
        app.output_mode,
        format!("Fetching balance for {address:#x}..."),
        async { native_balance(&client, address).await },
    )
    .await?;
    let formatted = format_units(balance, 18);
    let address = format!("{address:#x}");
    let wei = balance.to_string();

    render_balance_output(
        &chain.entry.key,
        &chain.entry.native_symbol,
        &chain.rpc_url,
        &address,
        &formatted,
        &wei,
    )
    .print(app.output_mode)
}

pub(crate) fn render_balance_output(
    chain_key: &str,
    native_symbol: &str,
    rpc_url: &str,
    address: &str,
    formatted: &str,
    wei: &str,
) -> CommandOutput {
    CommandOutput::new(
        balance_message(format!("{formatted} {native_symbol}"), address),
        json!({
            "address": address,
            "balance": formatted,
            "chain": chain_key,
            "native_symbol": native_symbol,
            "rpc_url": rpc_url,
            "wei": wei,
        }),
    )
    .compact(formatted.to_string())
    .markdown(format!(
        "- Chain: `{}`\n- Address: `{address}`\n- Balance: `{formatted} {}`",
        chain_key, native_symbol,
    ))
}
