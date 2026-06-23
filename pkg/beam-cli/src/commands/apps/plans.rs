use crate::{
    apps::{
        Error as AppError,
        model::{
            ActionPlan, ActionStep, AppManifest, AppPermissions, ChainOperation, InstalledApp,
        },
        permissions::{
            ensure_chain_scope_with_dynamic, normalize_dynamic_contracts,
            validate_dynamic_contracts,
        },
        store::now,
    },
    error::Result,
    runtime::BeamApp,
};

pub(super) async fn validate_guest_plan(
    app: &BeamApp,
    manifest: &AppManifest,
    installed: &InstalledApp,
    command_args: &[String],
    plan: &ActionPlan,
) -> Result<()> {
    let expected_command = command_args
        .first()
        .ok_or_else(|| AppError::InvalidGuestOutput {
            reason: "guest plan missing command".to_string(),
        })?;
    if !plan.command.starts_with(expected_command) {
        return Err(AppError::InvalidGuestOutput {
            reason: format!(
                "guest plan command `{}` does not match invocation `{expected_command}`",
                plan.command
            ),
        }
        .into());
    }
    if plan.app_id != manifest.id
        || plan.app_version != installed.active_version
        || plan.manifest_sha256 != installed.manifest_sha256
        || plan.wasm_sha256 != installed.module_sha256
    {
        return Err(AppError::InvalidGuestOutput {
            reason: "guest plan artifact identity does not match installed app".to_string(),
        }
        .into());
    }
    if plan.expires_at <= now() {
        return Err(AppError::InvalidGuestOutput {
            reason: "guest plan is already expired".to_string(),
        }
        .into());
    }

    let active_chain = app.active_chain().await?;
    if plan.chain != active_chain.entry.key {
        return Err(AppError::InvalidGuestOutput {
            reason: format!(
                "guest plan chain `{}` does not match active chain `{}`",
                plan.chain, active_chain.entry.key
            ),
        }
        .into());
    }
    if let Some(wallet) = plan.wallet.as_ref() {
        let active_wallet = format!("{:#x}", app.active_address().await?);
        if !wallet.eq_ignore_ascii_case(&active_wallet) {
            return Err(AppError::InvalidGuestOutput {
                reason: format!(
                    "guest plan wallet `{wallet}` does not match active wallet `{active_wallet}`"
                ),
            }
            .into());
        }
    }

    Ok(())
}

pub(super) fn validate_plan_permissions(
    permissions: &AppPermissions,
    plan: &ActionPlan,
) -> Result<()> {
    validate_dynamic_contracts(&plan.dynamic_contracts, &plan.chain)?;
    let dynamic_contracts = normalize_dynamic_contracts(&plan.dynamic_contracts);
    for step in &plan.steps {
        if let Some(target) = step.target.as_deref() {
            ensure_chain_scope_with_dynamic(
                permissions,
                &dynamic_contracts,
                &plan.chain,
                operation_for_step(step),
                Some(target),
                None,
                None,
            )?;
        }
        if let Some(selector) = step.selector.as_deref() {
            ensure_chain_scope_with_dynamic(
                permissions,
                &dynamic_contracts,
                &plan.chain,
                operation_for_step(step),
                None,
                Some(selector),
                None,
            )?;
        }
        if let Some(spender) = step.spender.as_deref() {
            ensure_chain_scope_with_dynamic(
                permissions,
                &dynamic_contracts,
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
