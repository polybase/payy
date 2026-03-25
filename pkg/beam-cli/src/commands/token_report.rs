use futures::{StreamExt, TryStreamExt, stream};
use serde_json::json;

use crate::{
    error::{Error, Result},
    evm::{erc20_balance, format_units, native_balance},
    human_output::sanitize_control_chars,
    output::{CommandOutput, with_loading},
    runtime::{BeamApp, parse_address},
    table::{render_markdown_table, render_table},
};

const TRACKED_TOKEN_BALANCE_CONCURRENCY: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TokenBalanceEntry {
    pub balance: String,
    pub decimals: u8,
    pub is_native: bool,
    pub label: String,
    pub token_address: Option<String>,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TokenBalanceReport {
    pub address: String,
    pub chain: String,
    pub native_symbol: String,
    pub rpc_url: String,
    pub tokens: Vec<TokenBalanceEntry>,
}

pub(crate) async fn load_token_balance_report(app: &BeamApp) -> Result<TokenBalanceReport> {
    let (chain, client) = app.active_chain_client().await?;
    let owner = app.active_address().await?;
    let address = format!("{owner:#x}");
    let tracked_tokens = app.tracked_tokens_for_chain(&chain.entry.key).await;

    let tokens = with_loading(
        app.output_mode,
        format!("Fetching balances for {owner:#x}..."),
        async {
            let mut tokens = Vec::new();
            let native = native_balance(&client, owner).await?;
            tokens.push(TokenBalanceEntry {
                balance: format_units(native, 18),
                decimals: 18,
                is_native: true,
                label: chain.entry.native_symbol.clone(),
                token_address: None,
                value: native.to_string(),
            });

            // Preserve the configured token order while limiting concurrent RPC calls.
            tokens.extend(
                stream::iter(tracked_tokens)
                    .map(|token| {
                        let client = client.clone();
                        async move {
                            let token_address = parse_address(&token.address)?;
                            let balance = erc20_balance(&client, token_address, owner).await?;
                            Ok::<_, Error>(TokenBalanceEntry {
                                balance: format_units(balance, token.decimals),
                                decimals: token.decimals,
                                is_native: false,
                                label: token.label,
                                token_address: Some(format!("{token_address:#x}")),
                                value: balance.to_string(),
                            })
                        }
                    })
                    .buffered(TRACKED_TOKEN_BALANCE_CONCURRENCY)
                    .try_collect::<Vec<_>>()
                    .await?,
            );

            Ok::<_, Error>(tokens)
        },
    )
    .await?;

    Ok(TokenBalanceReport {
        address,
        chain: chain.entry.key,
        native_symbol: chain.entry.native_symbol,
        rpc_url: chain.rpc_url,
        tokens,
    })
}

pub(crate) fn render_token_balance_report(report: &TokenBalanceReport) -> CommandOutput {
    let headers = ["token", "balance", "address"];
    let rows = report
        .tokens
        .iter()
        .map(|token| {
            vec![
                token.label.clone(),
                token.balance.clone(),
                token
                    .token_address
                    .clone()
                    .unwrap_or_else(|| "native".to_string()),
            ]
        })
        .collect::<Vec<_>>();
    let table = render_table(&headers, &rows);

    CommandOutput::new(
        format!(
            "Balances for {} on {}\n{table}",
            report.address, report.chain
        ),
        json!({
            "address": report.address.clone(),
            "chain": report.chain.clone(),
            "native_symbol": report.native_symbol.clone(),
            "rpc_url": report.rpc_url.clone(),
            "tokens": report.tokens.iter().map(|token| {
                json!({
                    "balance": token.balance.clone(),
                    "decimals": token.decimals,
                    "is_native": token.is_native,
                    "token": token.label.clone(),
                    "token_address": token.token_address.clone(),
                    "value": token.value.clone(),
                })
            }).collect::<Vec<_>>(),
        }),
    )
    .compact(
        report
            .tokens
            .iter()
            .map(|token| format!("{} {}", sanitize_control_chars(&token.label), token.balance))
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .markdown(format!(
        "Balances for `{}` on `{}`\n\n{}",
        report.address,
        report.chain,
        render_markdown_table(&headers, &rows),
    ))
}
