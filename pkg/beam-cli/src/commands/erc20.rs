// lint-long-file-override allow-max-lines=300
mod token_output;

use serde_json::json;
use web3::ethabi::StateMutability;

use crate::{
    abi::parse_function,
    cli::Erc20Action,
    commands::signing::{active_signer_for_intent, active_signing_address},
    error::{Error, Result},
    evm::{FunctionCall, erc20_balance, erc20_decimals, format_units, parse_units, send_function},
    human_output::sanitize_control_chars,
    output::{CommandOutput, with_loading, with_loading_handle},
    profiles::model::{ApprovalIntent, PublicSigningIntent, TokenTransferIntent},
    runtime::BeamApp,
    transaction::loading_message,
};

use self::token_output::{TokenWriteOutputConfig, print_token_write_output};

pub async fn run(app: &BeamApp, action: Erc20Action) -> Result<()> {
    match action {
        Erc20Action::Balance { token, address } => balance(app, &token, address.as_deref()).await,
        Erc20Action::Transfer { token, to, amount } => transfer(app, &token, &to, &amount).await,
        Erc20Action::Approve {
            token,
            spender,
            amount,
        } => approve(app, &token, &spender, &amount).await,
    }
}

async fn balance(app: &BeamApp, token: &str, address: Option<&str>) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let token = app.token_for_chain(token, &chain.entry.key).await?;
    let display_label = sanitize_control_chars(&token.label);
    let owner = match address {
        Some(address) => app.resolve_wallet_or_address(address).await?,
        None => app.active_address().await?,
    };
    let (decimals, balance) = with_loading(
        app.output_mode,
        format!("Fetching {display_label} balance for {owner:#x}..."),
        async {
            let decimals = token
                .decimals
                .unwrap_or(erc20_decimals(&client, token.address).await?);
            let balance = erc20_balance(&client, token.address, owner).await?;
            Ok::<_, Error>((decimals, balance))
        },
    )
    .await?;
    let formatted = format_units(balance, decimals);
    let owner = format!("{owner:#x}");
    let token_address = format!("{:#x}", token.address);
    let value = balance.to_string();

    render_balance_output(
        &chain.entry.key,
        &token.label,
        &token_address,
        &owner,
        &formatted,
        decimals,
        &value,
    )
    .print(app.output_mode)
}

pub(crate) fn render_balance_output(
    chain_key: &str,
    token_label: &str,
    token_address: &str,
    owner: &str,
    formatted: &str,
    decimals: u8,
    value: &str,
) -> CommandOutput {
    CommandOutput::new(
        format!(
            "{formatted} {}\nAddress: {owner}\nToken: {token_address}",
            sanitize_control_chars(token_label)
        ),
        json!({
            "address": owner,
            "balance": formatted,
            "chain": chain_key,
            "decimals": decimals,
            "token": token_label,
            "token_address": token_address,
            "value": value,
        }),
    )
    .compact(formatted.to_string())
}

async fn transfer(app: &BeamApp, token: &str, to: &str, amount: &str) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let token = app.token_for_chain(token, &chain.entry.key).await?;
    let token_label = sanitize_control_chars(&token.label);
    let to = app.resolve_wallet_or_address(to).await?;
    let decimals = match token.decimals {
        Some(decimals) => decimals,
        None => {
            with_loading(
                app.output_mode,
                format!("Fetching {token_label} token metadata..."),
                async { erc20_decimals(&client, token.address).await },
            )
            .await?
        }
    };
    let amount_value = parse_units(amount, usize::from(decimals))?;
    let wallet = active_signing_address(app).await?;
    let signer = active_signer_for_intent(
        app,
        PublicSigningIntent::Erc20Transfer(TokenTransferIntent {
            wallet: format!("{wallet:#x}"),
            chain: chain.entry.key.clone(),
            token: format!("{:#x}", token.address),
            recipient: format!("{to:#x}"),
            amount: amount_value.to_string(),
        }),
    )
    .await?;
    let function = parse_function("transfer(address,uint256)", StateMutability::NonPayable)?;
    let action = format!("transfer of {amount} {token_label} to {to:#x}");
    let execution = with_loading_handle(
        app.output_mode,
        format!("Sending {action} and waiting for confirmation..."),
        |loading| async move {
            send_function(
                &client,
                signer.as_ref(),
                FunctionCall {
                    args: &[format!("{to:#x}"), amount_value.to_string()],
                    contract: token.address,
                    function: &function,
                    value: 0u8.into(),
                },
                move |update| loading.set_message(loading_message(&action, &update)),
                tokio::signal::ctrl_c(),
            )
            .await
        },
    )
    .await?;

    print_token_write_output(
        app.output_mode,
        execution,
        TokenWriteOutputConfig {
            amount: amount.to_string(),
            chain_key: chain.entry.key.clone(),
            confirmed_summary: format!("Confirmed transfer of {amount} {token_label} to {to:#x}"),
            dropped_summary: format!(
                "Submitted transfer of {amount} {token_label} to {to:#x}, but the node no longer reports the transaction"
            ),
            pending_summary: format!(
                "Submitted transfer of {amount} {token_label} to {to:#x} and stopped waiting for confirmation"
            ),
            target_key: "to",
            target_value: format!("{to:#x}"),
            token_address: format!("{:#x}", token.address),
            token_label: token.label.clone(),
        },
    )
}

async fn approve(app: &BeamApp, token: &str, spender: &str, amount: &str) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let token = app.token_for_chain(token, &chain.entry.key).await?;
    let token_label = sanitize_control_chars(&token.label);
    let spender = app.resolve_wallet_or_address(spender).await?;
    let decimals = match token.decimals {
        Some(decimals) => decimals,
        None => {
            with_loading(
                app.output_mode,
                format!("Fetching {token_label} token metadata..."),
                async { erc20_decimals(&client, token.address).await },
            )
            .await?
        }
    };
    let amount_value = parse_units(amount, usize::from(decimals))?;
    let wallet = active_signing_address(app).await?;
    let signer = active_signer_for_intent(
        app,
        PublicSigningIntent::Erc20Approval(ApprovalIntent {
            wallet: format!("{wallet:#x}"),
            chain: chain.entry.key.clone(),
            token: format!("{:#x}", token.address),
            spender: format!("{spender:#x}"),
            amount: amount_value.to_string(),
        }),
    )
    .await?;
    let function = parse_function("approve(address,uint256)", StateMutability::NonPayable)?;
    let action = format!("approval of {amount} {token_label} for {spender:#x}");
    let execution = with_loading_handle(
        app.output_mode,
        format!("Sending {action} and waiting for confirmation..."),
        |loading| async move {
            send_function(
                &client,
                signer.as_ref(),
                FunctionCall {
                    args: &[format!("{spender:#x}"), amount_value.to_string()],
                    contract: token.address,
                    function: &function,
                    value: 0u8.into(),
                },
                move |update| loading.set_message(loading_message(&action, &update)),
                tokio::signal::ctrl_c(),
            )
            .await
        },
    )
    .await?;

    print_token_write_output(
        app.output_mode,
        execution,
        TokenWriteOutputConfig {
            amount: amount.to_string(),
            chain_key: chain.entry.key.clone(),
            confirmed_summary: format!(
                "Confirmed approval of {amount} {token_label} for {spender:#x}"
            ),
            dropped_summary: format!(
                "Submitted approval of {amount} {token_label} for {spender:#x}, but the node no longer reports the transaction"
            ),
            pending_summary: format!(
                "Submitted approval of {amount} {token_label} for {spender:#x} and stopped waiting for confirmation"
            ),
            target_key: "spender",
            target_value: format!("{spender:#x}"),
            token_address: format!("{:#x}", token.address),
            token_label: token.label.clone(),
        },
    )
}
