use crate::{
    apps::{
        Error as AppError,
        model::{
            ActionPlan, ActionStep, AppManifest, AppPermissions, ChainOperation, InstalledApp,
        },
        permissions::ensure_chain_scope,
    },
    error::Result,
    runtime::BeamApp,
};

pub(super) async fn plan_for_command(
    _app: &BeamApp,
    manifest: &AppManifest,
    _installed: &InstalledApp,
    args: &[String],
) -> Result<ActionPlan> {
    match (manifest.id.as_str(), args.first().map(String::as_str)) {
        (_, Some(command)) => Err(AppError::UnsupportedAppCommand {
            command: command.to_string(),
        }
        .into()),
        (_, None) => Err(AppError::UnsupportedAppCommand {
            command: "<missing>".to_string(),
        }
        .into()),
    }
}

pub(super) fn validate_plan_permissions(
    permissions: &AppPermissions,
    plan: &ActionPlan,
) -> Result<()> {
    for step in &plan.steps {
        if let Some(target) = step.target.as_deref() {
            ensure_chain_scope(
                permissions,
                &plan.chain,
                operation_for_step(step),
                Some(target),
                None,
                None,
            )?;
        }
        if let Some(selector) = step.selector.as_deref() {
            ensure_chain_scope(
                permissions,
                &plan.chain,
                operation_for_step(step),
                None,
                Some(selector),
                None,
            )?;
        }
        if let Some(spender) = step.spender.as_deref() {
            ensure_chain_scope(
                permissions,
                &plan.chain,
                operation_for_step(step),
                None,
                None,
                Some(spender),
            )?;
        }
    }

    Ok(())
}

fn operation_for_step(step: &ActionStep) -> ChainOperation {
    if step.kind == "erc20-approval" {
        ChainOperation::Erc20Approval
    } else {
        ChainOperation::SendTransaction
    }
}
