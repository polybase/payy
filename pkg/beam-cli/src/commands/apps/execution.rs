// lint-long-file-override allow-max-lines=300
mod tx_metadata;

use serde_json::{Value, json};

use crate::{
    apps::{
        Error as AppError,
        approvals::plan_hash,
        model::{ActionPlan, ActionStep, ApprovalFeeCap},
    },
    commands::signing::{active_signer_for_intent, active_signing_address},
    error::Result,
    evm::{
        CalldataTransaction, TransactionGasPolicy, erc20_allowance, send_calldata_with_fee_report,
        transaction_fee_json,
    },
    output::{
        CommandOutput, confirmed_transaction_message, dropped_transaction_message,
        pending_transaction_message, with_loading_handle,
    },
    profiles::model::{ActionGrantBinding, ActionGrantStep, AppActionIntent, PublicSigningIntent},
    runtime::{BeamApp, parse_address},
    signer::Signer,
    transaction::{TransactionExecution, loading_message},
};

use self::tx_metadata::{TransactionValue, parse_hex_data, parse_u256, transaction};

pub async fn execute_plan(
    app: &BeamApp,
    plan: &ActionPlan,
    fee_caps: &[ApprovalFeeCap],
) -> Result<CommandOutput> {
    execute_plan_with_approval(app, plan, fee_caps, None).await
}

pub async fn execute_plan_with_approval(
    app: &BeamApp,
    plan: &ActionPlan,
    fee_caps: &[ApprovalFeeCap],
    approval_id: Option<String>,
) -> Result<CommandOutput> {
    let wallet = active_signing_address(app).await?;
    let signer = active_signer_for_intent(
        app,
        PublicSigningIntent::AppActionPlan(app_intent(plan, format!("{wallet:#x}"), approval_id)?),
    )
    .await?;
    execute_plan_with_signer(app, plan, fee_caps, signer.as_ref()).await
}

pub(crate) async fn execute_plan_with_signer<S: Signer + ?Sized>(
    app: &BeamApp,
    plan: &ActionPlan,
    fee_caps: &[ApprovalFeeCap],
    signer: &S,
) -> Result<CommandOutput> {
    let executable = plan.steps.iter().any(|step| transaction(step).is_some());
    if !executable {
        return Ok(render_simulated_execution(plan));
    }

    let (chain, client) = app.active_chain_client().await?;
    let mut outputs = Vec::new();
    for (step_index, step) in plan.steps.iter().enumerate() {
        let Some(transaction) = transaction(step) else {
            continue;
        };
        if should_skip_approval(&client, signer.address(), step).await? {
            outputs.push(json!({
                "block_number": null,
                "state": "skipped",
                "status": null,
                "summary": format!("Skipped {}; allowance already sufficient", step.summary),
                "fee": null,
                "tx_hash": null,
            }));
            continue;
        }
        let to = parse_address(transaction.to()?)?;
        let data = parse_hex_data(transaction.data()?)?;
        let value = parse_u256(transaction.value().unwrap_or("0"))?;
        let gas = parse_gas_policy(step_index, &transaction, fee_caps)?;
        let action = step.summary.clone();
        let report = with_loading_handle(
            app.output_mode,
            format!("Sending {action} and waiting for confirmation..."),
            |loading| async {
                send_calldata_with_fee_report(
                    &client,
                    signer,
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
        let approval_ready = approval_step_ready(step, &report.execution);
        outputs.push(step_output(
            step,
            report.execution,
            transaction_fee_json(&report.gas),
        ));
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

fn app_intent(
    plan: &ActionPlan,
    wallet: String,
    approval_id: Option<String>,
) -> Result<AppActionIntent> {
    Ok(AppActionIntent {
        app_id: plan.app_id.clone(),
        app_version: plan.app_version.clone(),
        registry_url: None,
        manifest_digest: plan.manifest_sha256.clone(),
        module_digest: plan.wasm_sha256.clone(),
        command: plan.command.clone(),
        wallet,
        chain: plan.chain.clone(),
        plan_hash: plan_hash(plan)?,
        expires_at: plan.expires_at,
        bindings: plan
            .bindings
            .iter()
            .map(|binding| ActionGrantBinding {
                key: binding.key.clone(),
                value: binding.value.clone(),
            })
            .collect(),
        constraints: plan.constraints.clone(),
        steps: plan
            .steps
            .iter()
            .map(|step| ActionGrantStep {
                kind: step.kind.clone(),
                target: step.target.clone(),
                selector: step.selector.clone(),
                spender: step.spender.clone(),
                value: step.value.clone(),
                metadata: step.metadata.clone(),
            })
            .collect(),
        approval_id,
    })
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

fn step_output(step: &ActionStep, execution: TransactionExecution, fee: Value) -> Value {
    match execution {
        TransactionExecution::Confirmed(outcome) => json!({
            "block_number": outcome.block_number,
            "fee": fee,
            "state": "confirmed",
            "status": outcome.status,
            "summary": confirmed_transaction_message(&step.summary, &outcome.tx_hash, outcome.block_number),
            "tx_hash": outcome.tx_hash,
        }),
        TransactionExecution::Pending(pending) => json!({
            "block_number": pending.block_number,
            "fee": fee,
            "state": "pending",
            "status": null,
            "summary": pending_transaction_message(&step.summary, &pending.tx_hash, pending.block_number),
            "tx_hash": pending.tx_hash,
        }),
        TransactionExecution::Dropped(dropped) => json!({
            "block_number": dropped.block_number,
            "fee": fee,
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

fn parse_gas_policy(
    step_index: usize,
    transaction: &TransactionValue<'_>,
    fee_caps: &[ApprovalFeeCap],
) -> Result<Option<TransactionGasPolicy>> {
    let fee_cap = fee_caps
        .iter()
        .find(|fee_cap| fee_cap.step_index == step_index)
        .ok_or(AppError::ApprovalFeeCapMissing { step_index })?;
    Ok(Some(TransactionGasPolicy {
        gas_limit: transaction.gas_limit().map(parse_u256).transpose()?,
        max_network_fee: Some(parse_u256(&fee_cap.approved_max_total_fee_wei)?),
    }))
}
