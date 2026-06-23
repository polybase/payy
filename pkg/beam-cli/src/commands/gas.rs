// lint-long-file-override allow-max-lines=400
use serde_json::{Value, json};
use web3::ethabi::StateMutability;

use crate::{
    abi::parse_function,
    cli::{Erc20GasAction, GasAction, SendArgs, TransferArgs},
    commands::call::{parse_transaction_value, resolve_address_args},
    error::Result,
    evm::{
        EvmFeeEstimate, EvmFeeMode, FunctionCall, TransactionGas, erc20_decimals,
        estimate_function_gas, estimate_native_gas, format_units, parse_units,
    },
    human_output::sanitize_control_chars,
    output::{CommandOutput, with_loading},
    runtime::{BeamApp, parse_address},
};

pub async fn run(app: &BeamApp, action: GasAction) -> Result<()> {
    match action {
        GasAction::Transfer(args) => estimate_transfer(app, args).await,
        GasAction::Erc20 { action } => estimate_erc20(app, action).await,
        GasAction::Send(args) => estimate_send(app, args).await,
    }
}

async fn estimate_transfer(app: &BeamApp, args: TransferArgs) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let from = app.active_address().await?;
    let to = app.resolve_wallet_or_address(&args.to).await?;
    let amount = parse_units(&args.amount, 18)?;
    let gas = with_loading(
        app.output_mode,
        format!("Estimating gas for transfer to {to:#x}..."),
        async { estimate_native_gas(&client, from, to, amount).await },
    )
    .await?;

    render_gas_output(GasOutputConfig {
        chain_key: &chain.entry.key,
        default_summary: format!(
            "Estimated gas for transfer of {} {} to {to:#x}",
            args.amount, chain.entry.native_symbol
        ),
        extra: json!({
            "amount": args.amount,
            "from": format!("{from:#x}"),
            "kind": "transfer",
            "to": format!("{to:#x}"),
        }),
        gas,
        native_symbol: &chain.entry.native_symbol,
    })
    .print(app.output_mode)
}

async fn estimate_erc20(app: &BeamApp, action: Erc20GasAction) -> Result<()> {
    match action {
        Erc20GasAction::Transfer { token, to, amount } => {
            estimate_erc20_write(app, token, to, amount, Erc20GasKind::Transfer).await
        }
        Erc20GasAction::Approve {
            token,
            spender,
            amount,
        } => estimate_erc20_write(app, token, spender, amount, Erc20GasKind::Approve).await,
    }
}

async fn estimate_erc20_write(
    app: &BeamApp,
    token: String,
    target: String,
    amount: String,
    kind: Erc20GasKind,
) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let from = app.active_address().await?;
    let token = app.token_for_chain(&token, &chain.entry.key).await?;
    let token_label = sanitize_control_chars(&token.label);
    let target = app.resolve_wallet_or_address(&target).await?;
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
    let amount_value = parse_units(&amount, usize::from(decimals))?;
    let function = parse_function(kind.signature(), StateMutability::NonPayable)?;
    let function_args = vec![format!("{target:#x}"), amount_value.to_string()];
    let gas = with_loading(
        app.output_mode,
        format!(
            "Estimating gas for {} of {amount} {token_label}...",
            kind.noun()
        ),
        async {
            estimate_function_gas(
                &client,
                from,
                FunctionCall {
                    args: &function_args,
                    contract: token.address,
                    function: &function,
                    value: 0u8.into(),
                },
            )
            .await
        },
    )
    .await?;
    let mut extra = json!({
        "amount": amount,
        "from": format!("{from:#x}"),
        "kind": kind.json_kind(),
        "token": token.label,
        "token_address": format!("{:#x}", token.address),
    });
    if let Some(extra) = extra.as_object_mut() {
        extra.insert(
            kind.target_key().to_string(),
            Value::String(format!("{target:#x}")),
        );
    }

    render_gas_output(GasOutputConfig {
        chain_key: &chain.entry.key,
        default_summary: format!(
            "Estimated gas for {} of {amount} {token_label} {} {target:#x}",
            kind.noun(),
            kind.preposition()
        ),
        extra,
        gas,
        native_symbol: &chain.entry.native_symbol,
    })
    .print(app.output_mode)
}

async fn estimate_send(app: &BeamApp, args: SendArgs) -> Result<()> {
    let (chain, client) = app.active_chain_client().await?;
    let from = app.active_address().await?;
    let value_display = args.value.clone().unwrap_or_else(|| "0".to_string());
    let value = parse_transaction_value(args.value.as_deref())?;
    let contract = parse_address(&args.call.contract)?;
    let function = parse_function(&args.call.function_sig, StateMutability::NonPayable)?;
    let call_args = resolve_address_args(app, &function, &args.call.args).await?;
    let gas = with_loading(
        app.output_mode,
        format!("Estimating gas for transaction to {contract:#x}..."),
        async {
            estimate_function_gas(
                &client,
                from,
                FunctionCall {
                    args: &call_args,
                    contract,
                    function: &function,
                    value,
                },
            )
            .await
        },
    )
    .await?;

    render_gas_output(GasOutputConfig {
        chain_key: &chain.entry.key,
        default_summary: if value.is_zero() {
            format!("Estimated gas for transaction to {contract:#x}")
        } else {
            format!(
                "Estimated gas for transaction to {contract:#x} with {value_display} {}",
                chain.entry.native_symbol
            )
        },
        extra: json!({
            "contract": format!("{contract:#x}"),
            "from": format!("{from:#x}"),
            "kind": "send",
            "signature": args.call.function_sig,
            "value": value_display,
        }),
        gas,
        native_symbol: &chain.entry.native_symbol,
    })
    .print(app.output_mode)
}

struct GasOutputConfig<'a> {
    chain_key: &'a str,
    default_summary: String,
    extra: serde_json::Value,
    gas: TransactionGas,
    native_symbol: &'a str,
}

#[derive(Clone, Copy)]
enum Erc20GasKind {
    Approve,
    Transfer,
}

#[cfg(test)]
#[path = "gas/tests.rs"]
mod tests;

impl Erc20GasKind {
    fn json_kind(self) -> &'static str {
        match self {
            Self::Approve => "erc20_approve",
            Self::Transfer => "erc20_transfer",
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Self::Approve => "approval",
            Self::Transfer => "transfer",
        }
    }

    fn preposition(self) -> &'static str {
        match self {
            Self::Approve => "for",
            Self::Transfer => "to",
        }
    }

    fn signature(self) -> &'static str {
        match self {
            Self::Approve => "approve(address,uint256)",
            Self::Transfer => "transfer(address,uint256)",
        }
    }

    fn target_key(self) -> &'static str {
        match self {
            Self::Approve => "spender",
            Self::Transfer => "to",
        }
    }
}

fn render_gas_output(config: GasOutputConfig<'_>) -> CommandOutput {
    let fee = config.gas.fee();
    let fee_display = format_units(fee, 18);
    let mut value = json!({
        "chain": config.chain_key,
        "estimated_fee": fee_display,
        "estimated_fee_wei": fee.to_string(),
        "fee_mode": fee_mode_label(&config.gas.fee),
        "gas_limit": config.gas.gas_limit.to_string(),
        "max_fee_per_gas": config.gas.gas_price_for_display().to_string(),
        "native_symbol": config.native_symbol,
    });
    if let Some(output) = value.as_object_mut() {
        match config.gas.fee {
            EvmFeeEstimate::Legacy { gas_price } => {
                output.insert("gas_price".to_string(), json!(gas_price.to_string()));
            }
            EvmFeeEstimate::Eip1559 {
                max_priority_fee_per_gas,
                ..
            } => {
                output.insert(
                    "max_priority_fee_per_gas".to_string(),
                    json!(max_priority_fee_per_gas.to_string()),
                );
            }
        }
    }

    if let Some(output) = value.as_object_mut()
        && let Some(extra) = config.extra.as_object()
    {
        output.extend(extra.clone());
    }

    CommandOutput::new(
        format!(
            "{}\nEstimated fee: {} {} ({} wei)\nGas limit: {}\nFee mode: {}\nMax fee per gas: {} wei",
            config.default_summary,
            fee_display,
            config.native_symbol,
            fee,
            config.gas.gas_limit,
            fee_mode_label(&config.gas.fee),
            config.gas.gas_price_for_display(),
        ),
        value,
    )
    .compact(fee_display.clone())
    .markdown(format!(
        "- Chain: `{}`\n- Estimated fee: `{}` `{}` (`{}` wei)\n- Gas limit: `{}`\n- Fee mode: `{}`\n- Max fee per gas: `{}` wei",
        config.chain_key,
        fee_display,
        config.native_symbol,
        fee,
        config.gas.gas_limit,
        fee_mode_label(&config.gas.fee),
        config.gas.gas_price_for_display(),
    ))
}

fn fee_mode_label(fee: &EvmFeeEstimate) -> &'static str {
    match fee.mode() {
        EvmFeeMode::Legacy => "legacy",
        EvmFeeMode::Eip1559 => "eip1559",
    }
}
