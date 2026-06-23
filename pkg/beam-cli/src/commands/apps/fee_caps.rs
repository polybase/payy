// lint-long-file-override allow-max-lines=300
use contracts::U256;
use serde_json::Value;

use super::{prompt::approve_interactively, render};
use crate::{
    apps::{
        Error as AppError,
        model::{ActionPlan, ActionStep, ApprovalFeeCap, ApprovalRecord},
    },
    error::{Error, Result},
    evm::{EvmFeeEstimate, TransactionGas, TransactionGasPolicy, resolve_transaction_gas},
    output::OutputMode,
    runtime::{BeamApp, parse_address},
};

const DEFAULT_APPROVAL_FEE_CAP_MULTIPLIER: u64 = 2;

pub(super) async fn approval_fee_caps(
    app: &BeamApp,
    plan: &ActionPlan,
    user_max_network_fee: Option<U256>,
) -> Result<Vec<ApprovalFeeCap>> {
    let executable_steps = plan
        .steps
        .iter()
        .any(|step| transaction_metadata(step).is_some());
    if !executable_steps {
        return Ok(Vec::new());
    }

    let (_, client) = app.active_chain_client().await?;
    let from = app.active_address().await?;
    let mut caps = Vec::new();
    for (step_index, step) in plan.steps.iter().enumerate() {
        let Some(transaction) = transaction_metadata(step) else {
            continue;
        };
        let to = parse_address(transaction.string("to")?)?;
        let data = parse_hex_data(transaction.string("data")?)?;
        let value = transaction
            .optional_string("value")
            .map(parse_u256)
            .transpose()?
            .unwrap_or_else(U256::zero);
        let gas_limit = transaction
            .optional_string("gas_limit")
            .map(parse_u256)
            .transpose()?;
        let gas = resolve_transaction_gas(
            &client,
            from,
            to,
            &data,
            value,
            Some(TransactionGasPolicy {
                gas_limit,
                max_network_fee: user_max_network_fee,
            }),
        )
        .await?;
        let approved_max_total_fee = user_max_network_fee.unwrap_or_else(|| {
            gas.max_network_fee() * U256::from(DEFAULT_APPROVAL_FEE_CAP_MULTIPLIER)
        });
        caps.push(approval_fee_cap(step_index, gas, approved_max_total_fee));
    }

    Ok(caps)
}

fn approval_fee_cap(
    step_index: usize,
    gas: TransactionGas,
    approved_max_total_fee: U256,
) -> ApprovalFeeCap {
    let approved_max_fee_per_gas = if gas.gas_limit.is_zero() {
        U256::zero()
    } else {
        approved_max_total_fee / gas.gas_limit
    };
    let (fee_mode, approved_max_priority_fee_per_gas) = match gas.fee {
        EvmFeeEstimate::Legacy { .. } => ("legacy", None),
        EvmFeeEstimate::Eip1559 {
            max_priority_fee_per_gas,
            ..
        } => ("eip1559", Some(max_priority_fee_per_gas.to_string())),
    };

    ApprovalFeeCap {
        step_index,
        approved_gas_limit: gas.gas_limit.to_string(),
        approved_max_fee_per_gas: approved_max_fee_per_gas.to_string(),
        approved_max_total_fee_wei: approved_max_total_fee.to_string(),
        fee_mode: fee_mode.to_string(),
        approved_max_priority_fee_per_gas,
    }
}

pub(super) fn parse_max_network_fee(value: &str) -> Result<U256> {
    parse_u256(value)
}

pub(super) fn max_network_fee_arg(
    cli_value: Option<&str>,
    args: &[String],
) -> Result<Option<U256>> {
    let trailing_value = trailing_max_network_fee_arg(args)?;
    match (cli_value, trailing_value.as_deref()) {
        (Some(value), Some(trailing)) if value != trailing => Err(AppError::InvalidHostRequest {
            reason: "conflicting --max-network-fee-wei values".to_string(),
        }
        .into()),
        (Some(value), _) | (_, Some(value)) => Ok(Some(parse_max_network_fee(value)?)),
        (None, None) => Ok(None),
    }
}

pub(super) async fn approval_fee_caps_for_execution(
    app: &BeamApp,
    approval: &ApprovalRecord,
    max_network_fee_wei: Option<&str>,
) -> Result<Vec<ApprovalFeeCap>> {
    if !approval.fee_caps.is_empty() {
        return Ok(approval.fee_caps.clone());
    }
    if app.output_mode != OutputMode::Default {
        return Err(AppError::ApprovalNeedsFeeCaps {
            approval_id: approval.id.clone(),
        }
        .into());
    }

    let max_network_fee = max_network_fee_wei.map(parse_max_network_fee).transpose()?;
    let fee_caps = approval_fee_caps(app, &approval.plan, max_network_fee).await?;
    approve_interactively(&render::render_plan_with_fee_caps(
        &approval.plan,
        &fee_caps,
    ))?;
    Ok(fee_caps)
}

fn trailing_max_network_fee_arg(args: &[String]) -> Result<Option<String>> {
    let mut value = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if let Some(arg_value) = arg.strip_prefix("--max-network-fee-wei=") {
            value = merge_max_network_fee_arg(value, arg_value)?;
            index += 1;
            continue;
        }
        if arg == "--max-network-fee-wei" {
            let Some(arg_value) = args.get(index + 1) else {
                return Err(AppError::InvalidHostRequest {
                    reason: "--max-network-fee-wei requires a value".to_string(),
                }
                .into());
            };
            value = merge_max_network_fee_arg(value, arg_value)?;
            index += 2;
            continue;
        }
        index += 1;
    }
    Ok(value)
}

fn merge_max_network_fee_arg(existing: Option<String>, next: &str) -> Result<Option<String>> {
    if let Some(existing) = existing
        && existing != next
    {
        return Err(AppError::InvalidHostRequest {
            reason: "conflicting --max-network-fee-wei values".to_string(),
        }
        .into());
    }
    Ok(Some(next.to_string()))
}

fn transaction_metadata(step: &ActionStep) -> Option<TransactionMetadata<'_>> {
    step.metadata
        .get("transaction")
        .and_then(Value::as_object)
        .map(TransactionMetadata)
}

struct TransactionMetadata<'a>(&'a serde_json::Map<String, Value>);

impl TransactionMetadata<'_> {
    fn string(&self, key: &str) -> Result<&str> {
        self.optional_string(key).ok_or_else(|| {
            Error::App(AppError::InvalidHostRequest {
                reason: format!("transaction missing {key}"),
            })
        })
    }

    fn optional_string(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }
}

fn parse_hex_data(value: &str) -> Result<Vec<u8>> {
    hex::decode(value.strip_prefix("0x").unwrap_or(value)).map_err(|_| Error::InvalidHexData {
        value: value.to_string(),
    })
}

fn parse_u256(value: &str) -> Result<U256> {
    if let Some(value) = value.strip_prefix("0x") {
        return U256::from_str_radix(value, 16).map_err(|_| Error::InvalidNumber {
            value: value.to_string(),
        });
    }
    U256::from_dec_str(value).map_err(|_| Error::InvalidNumber {
        value: value.to_string(),
    })
}
