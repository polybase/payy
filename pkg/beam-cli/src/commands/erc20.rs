// lint-long-file-override allow-max-lines=300
use serde_json::{Value, json};
use web3::ethabi::StateMutability;

use crate::{
    abi::parse_function,
    cli::Erc20Action,
    commands::signing::prompt_active_signer,
    error::{Error, Result},
    evm::{FunctionCall, erc20_balance, erc20_decimals, format_units, parse_units, send_function},
    human_output::sanitize_control_chars,
    output::{
        CommandOutput, OutputMode, confirmed_transaction_message, dropped_transaction_message,
        pending_transaction_message, with_loading, with_loading_handle,
    },
    runtime::BeamApp,
    transaction::{TransactionExecution, loading_message},
};

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
    let signer = prompt_active_signer(app).await?;
    let function = parse_function("transfer(address,uint256)", StateMutability::NonPayable)?;
    let action = format!("transfer of {amount} {token_label} to {to:#x}");
    let execution = with_loading_handle(
        app.output_mode,
        format!("Sending {action} and waiting for confirmation..."),
        |loading| async move {
            send_function(
                &client,
                &signer,
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
    let signer = prompt_active_signer(app).await?;
    let function = parse_function("approve(address,uint256)", StateMutability::NonPayable)?;
    let action = format!("approval of {amount} {token_label} for {spender:#x}");
    let execution = with_loading_handle(
        app.output_mode,
        format!("Sending {action} and waiting for confirmation..."),
        |loading| async move {
            send_function(
                &client,
                &signer,
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

struct TokenWriteOutputConfig {
    amount: String,
    chain_key: String,
    confirmed_summary: String,
    dropped_summary: String,
    pending_summary: String,
    target_key: &'static str,
    target_value: String,
    token_address: String,
    token_label: String,
}

fn print_token_write_output(
    output_mode: OutputMode,
    execution: TransactionExecution,
    config: TokenWriteOutputConfig,
) -> Result<()> {
    let (default, state, block_number, status, tx_hash) = match execution {
        TransactionExecution::Confirmed(outcome) => (
            confirmed_transaction_message(
                config.confirmed_summary,
                &outcome.tx_hash,
                outcome.block_number,
            ),
            "confirmed",
            outcome.block_number,
            outcome.status,
            outcome.tx_hash,
        ),
        TransactionExecution::Pending(pending) => (
            pending_transaction_message(
                config.pending_summary,
                &pending.tx_hash,
                pending.block_number,
            ),
            "pending",
            pending.block_number,
            None,
            pending.tx_hash,
        ),
        TransactionExecution::Dropped(dropped) => (
            dropped_transaction_message(
                config.dropped_summary,
                &dropped.tx_hash,
                dropped.block_number,
            ),
            "dropped",
            dropped.block_number,
            None,
            dropped.tx_hash,
        ),
    };

    let mut value = json!({
        "amount": config.amount,
        "block_number": block_number,
        "chain": config.chain_key,
        "state": state,
        "status": status,
        "token": config.token_label,
        "token_address": config.token_address,
        "tx_hash": tx_hash.clone(),
    });
    value.as_object_mut().expect("token write output").insert(
        config.target_key.to_string(),
        Value::String(config.target_value),
    );

    CommandOutput::new(default, value)
        .compact(tx_hash)
        .print(output_mode)
}
