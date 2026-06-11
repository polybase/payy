// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use serde_json::{Value, json};

use crate::{
    apps::{
        Error as AppError,
        model::{ActionPlan, ActionStep},
    },
    commands::signing::prompt_active_signer,
    error::{Error, Result},
    evm::{CalldataTransaction, TransactionGas, erc20_allowance, send_calldata_with_gas},
    output::{
        CommandOutput, confirmed_transaction_message, dropped_transaction_message,
        pending_transaction_message, with_loading_handle,
    },
    runtime::{BeamApp, parse_address},
    signer::Signer,
    transaction::{TransactionExecution, loading_message},
};

pub async fn execute_plan(app: &BeamApp, plan: &ActionPlan) -> Result<CommandOutput> {
    let executable = plan.steps.iter().any(|step| transaction(step).is_some());
    if !executable {
        return Ok(render_simulated_execution(plan));
    }

    let (chain, client) = app.active_chain_client().await?;
    let signer = prompt_active_signer(app).await?;
    let mut outputs = Vec::new();
    for step in &plan.steps {
        let Some(transaction) = transaction(step) else {
            continue;
        };
        if should_skip_approval(&client, signer.address(), step).await? {
            outputs.push(json!({
                "block_number": null,
                "state": "skipped",
                "status": null,
                "summary": format!("Skipped {}; allowance already sufficient", step.summary),
                "tx_hash": null,
            }));
            continue;
        }
        let to = parse_address(transaction.to()?)?;
        let data = parse_hex_data(transaction.data()?)?;
        let value = parse_u256(transaction.value().unwrap_or("0"))?;
        let gas = parse_gas(&transaction)?;
        let action = step.summary.clone();
        let execution = with_loading_handle(
            app.output_mode,
            format!("Sending {action} and waiting for confirmation..."),
            |loading| async {
                send_calldata_with_gas(
                    &client,
                    &signer,
                    CalldataTransaction {
                        data,
                        gas,
                        to,
                        value,
                    },
                    move |update| loading.set_message(loading_message(&action, &update)),
                    tokio::signal::ctrl_c(),
                )
                .await
            },
        )
        .await?;
        let approval_ready = approval_step_ready(step, &execution);
        outputs.push(step_output(step, execution));
        if step.kind == "erc20-approval" && !approval_ready {
            break;
        }
    }

    Ok(CommandOutput::new(
        render_execution_summary(plan, &outputs),
        json!({
            "app": plan.app_id,
            "chain": chain.entry.key,
            "command": plan.command,
            "state": aggregate_state(&outputs),
            "steps": outputs,
        }),
    ))
}

fn approval_step_ready(step: &ActionStep, execution: &TransactionExecution) -> bool {
    if step.kind != "erc20-approval" {
        return true;
    }
    matches!(
        execution,
        TransactionExecution::Confirmed(outcome) if outcome.status.unwrap_or(1) != 0
    )
}

fn render_simulated_execution(plan: &ActionPlan) -> CommandOutput {
    CommandOutput::new(
        format!("Executed app action: {}", plan.command),
        json!({
            "app": plan.app_id,
            "chain": plan.chain,
            "command": plan.command,
            "state": "executed",
            "steps": plan.steps,
        }),
    )
}

fn render_execution_summary(plan: &ActionPlan, outputs: &[Value]) -> String {
    let mut lines = vec![format!("Executed app action: {}", plan.command)];
    for output in outputs {
        let summary = output["summary"].as_str().unwrap_or("transaction");
        let tx_hash = output["tx_hash"].as_str().unwrap_or("unknown");
        let state = output["state"].as_str().unwrap_or("unknown");
        lines.push(format!("  - {summary}: {state} ({tx_hash})"));
    }
    lines.join("\n")
}

fn step_output(step: &ActionStep, execution: TransactionExecution) -> Value {
    match execution {
        TransactionExecution::Confirmed(outcome) => json!({
            "block_number": outcome.block_number,
            "state": "confirmed",
            "status": outcome.status,
            "summary": confirmed_transaction_message(&step.summary, &outcome.tx_hash, outcome.block_number),
            "tx_hash": outcome.tx_hash,
        }),
        TransactionExecution::Pending(pending) => json!({
            "block_number": pending.block_number,
            "state": "pending",
            "status": null,
            "summary": pending_transaction_message(&step.summary, &pending.tx_hash, pending.block_number),
            "tx_hash": pending.tx_hash,
        }),
        TransactionExecution::Dropped(dropped) => json!({
            "block_number": dropped.block_number,
            "state": "dropped",
            "status": null,
            "summary": dropped_transaction_message(&step.summary, &dropped.tx_hash, dropped.block_number),
            "tx_hash": dropped.tx_hash,
        }),
    }
}

fn aggregate_state(outputs: &[Value]) -> &'static str {
    if outputs.iter().any(|output| output["state"] == "dropped") {
        "dropped"
    } else if outputs.iter().any(|output| output["state"] == "pending") {
        "pending"
    } else if outputs.iter().all(|output| output["state"] == "skipped") {
        "skipped"
    } else {
        "confirmed"
    }
}

async fn should_skip_approval(
    client: &contracts::Client,
    wallet: contracts::Address,
    step: &ActionStep,
) -> Result<bool> {
    if step.kind != "erc20-approval" {
        return Ok(false);
    }
    let (Some(target), Some(spender), Some(value)) = (
        step.target.as_deref(),
        step.spender.as_deref(),
        step.value.as_deref(),
    ) else {
        return Ok(false);
    };
    let required = parse_u256(value)?;
    let allowance = erc20_allowance(
        client,
        parse_address(target)?,
        wallet,
        parse_address(spender)?,
    )
    .await?;

    Ok(allowance >= required)
}

fn transaction(step: &ActionStep) -> Option<TransactionValue<'_>> {
    step.metadata
        .get("transaction")
        .and_then(Value::as_object)
        .map(TransactionValue)
}

struct TransactionValue<'a>(&'a serde_json::Map<String, Value>);

impl TransactionValue<'_> {
    fn data(&self) -> Result<&str> {
        self.string("data")
    }

    fn gas_limit(&self) -> Option<&str> {
        self.optional_string("gas_limit")
    }

    fn gas_price(&self) -> Option<&str> {
        self.optional_string("gas_price")
    }

    fn to(&self) -> Result<&str> {
        self.string("to")
    }

    fn value(&self) -> Option<&str> {
        self.optional_string("value")
    }

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

fn parse_gas(transaction: &TransactionValue<'_>) -> Result<Option<TransactionGas>> {
    match (transaction.gas_limit(), transaction.gas_price()) {
        (Some(gas_limit), Some(gas_price)) => Ok(Some(TransactionGas {
            gas_limit: parse_u256(gas_limit)?,
            gas_price: parse_u256(gas_price)?,
        })),
        _ => Ok(None),
    }
}

fn parse_hex_data(value: &str) -> Result<Vec<u8>> {
    hex::decode(value.strip_prefix("0x").unwrap_or(value)).map_err(|_| Error::InvalidHexData {
        value: value.to_string(),
    })
}

fn parse_u256(value: &str) -> Result<contracts::U256> {
    if let Some(value) = value.strip_prefix("0x") {
        return Ok(contracts::U256::from_str_radix(value, 16).context("parse hex u256")?);
    }
    Ok(value
        .parse::<contracts::U256>()
        .context("parse decimal u256")?)
}
