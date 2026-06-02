use serde_json::{Value, json};

use crate::{
    apps::model::{ActionPlan, AppManifest, AppPermissions, ApprovalRecord},
    output::CommandOutput,
};

pub(super) fn render_install_summary(
    manifest: &AppManifest,
    module_sha256: &str,
    registry_url: &str,
) -> String {
    format!(
        "Install {} {}?\nPublisher: {}\nSource: {}\nWASM digest: {}\n\n{}",
        manifest.display_name,
        manifest.version,
        manifest.publisher,
        registry_url,
        module_sha256,
        render_permissions(&manifest.permissions)
    )
}

pub(super) fn render_manifest_info(manifest: &AppManifest) -> String {
    format!(
        "{} {}\nPublisher: {}\n{}\n\n{}",
        manifest.display_name,
        manifest.version,
        manifest.publisher,
        manifest.description,
        render_permissions(&manifest.permissions)
    )
}

pub(super) fn render_permissions(permissions: &AppPermissions) -> String {
    let mut lines = Vec::new();
    lines.push("Network:".to_string());
    if permissions.http.is_empty() {
        lines.push("  - none".to_string());
    } else {
        for http in &permissions.http {
            lines.push(format!("  - {}", http.url));
        }
    }
    lines.push("Contracts:".to_string());
    if permissions.chains.is_empty() {
        lines.push("  - none".to_string());
    } else {
        for chain in &permissions.chains {
            lines.push(format!(
                "  - {}: contracts {}, selectors {}, spenders {}",
                chain.chain,
                scope_label(chain.contracts.as_deref(), "any contract"),
                scope_label(chain.selectors.as_deref(), "any selector"),
                scope_label(chain.spenders.as_deref(), "any spender"),
            ));
        }
    }
    lines.push("Wallet actions:".to_string());
    lines.push(format!(
        "  - balances: {}\n  - transaction proposals: {}\n  - erc20 approvals: {}",
        permissions.wallet.read_balances,
        permissions.wallet.propose_transactions,
        permissions.wallet.erc20_approval
    ));
    lines.push(format!(
        "Storage:\n  - app-local: {}",
        permissions.storage.app_local
    ));
    if !permissions.privacy.is_empty() {
        lines.push(format!("Privacy:\n  - {:?}", permissions.privacy));
    }
    lines.join("\n")
}

fn scope_label(scope: Option<&[String]>, wildcard: &str) -> String {
    scope
        .map(|values| values.join(", "))
        .unwrap_or_else(|| wildcard.to_string())
}

pub(super) fn render_app_help(manifest: &AppManifest) -> String {
    let mut lines = vec![format!("{} commands:", manifest.display_name)];
    for command in &manifest.commands {
        lines.push(format!("  {} - {}", command.name, command.about));
    }
    lines.join("\n")
}

pub(super) fn render_plan(plan: &ActionPlan) -> String {
    let mut lines = vec![
        format!("App: {} {}", plan.app_id, plan.app_version),
        format!("Chain: {}", plan.chain),
        "Action:".to_string(),
    ];
    for step in &plan.steps {
        lines.push(format!("  - {}", step.summary));
    }
    lines.push(format!("Expires at: {}", plan.expires_at));
    lines.join("\n")
}

pub(super) fn render_approval(record: &ApprovalRecord) -> String {
    format!(
        "Approval: {}\nStatus: {:?}\nPlan hash: {}\n{}",
        record.id,
        record.status,
        record.plan_hash,
        render_plan(&record.plan)
    )
}

pub(super) fn render_approval_created(record: &ApprovalRecord) -> CommandOutput {
    CommandOutput::new(
        format!(
            "{}\nApprove with: beam apps approvals approve {} --execute",
            render_approval(record),
            record.id
        ),
        approval_json(record),
    )
}

pub(super) fn render_execution(plan: &ActionPlan) -> CommandOutput {
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

pub(super) fn render_permission_diff(current: &AppManifest, next: &AppManifest) -> String {
    format!(
        "Update {} {} -> {} changes permissions.\n\nCurrent:\n{}\n\nNext:\n{}",
        current.display_name,
        current.version,
        next.version,
        render_permissions(&current.permissions),
        render_permissions(&next.permissions)
    )
}

pub(super) fn manifest_json(manifest: &AppManifest) -> Value {
    json!({
        "id": manifest.id,
        "name": manifest.display_name,
        "version": manifest.version,
        "publisher": manifest.publisher,
        "description": manifest.description,
        "permissions": permissions_json(&manifest.permissions),
    })
}

pub(super) fn permissions_json(permissions: &AppPermissions) -> Value {
    serde_json::to_value(permissions).unwrap_or_else(|_| json!({}))
}

pub(super) fn approval_json(approval: &ApprovalRecord) -> Value {
    serde_json::to_value(approval).unwrap_or_else(|_| json!({}))
}
