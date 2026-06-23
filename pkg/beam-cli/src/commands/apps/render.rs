// lint-long-file-override allow-max-lines=300
use serde_json::{Value, json};

use crate::{
    apps::model::{
        ActionPlan, AppCommand, AppCommandExample, AppCommandParameter, AppManifest,
        AppPermissions, ApprovalFeeCap, ApprovalRecord,
    },
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
        "  - balances: {}\n  - transaction proposals: {}\n  - erc20 approvals: {}\n  - typed-data signing: {}",
        permissions.wallet.read_balances,
        permissions.wallet.propose_transactions,
        permissions.wallet.erc20_approval,
        permissions.wallet.sign_typed_data
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

pub(super) fn render_app_command_help(manifest: &AppManifest, command: &AppCommand) -> String {
    let mut lines = Vec::new();
    lines.push(format!("{} {}", manifest.display_name, command.name));
    lines.push(command.about.clone());
    lines.push(String::new());
    lines.push(format!(
        "Usage: {}",
        command_usage(command).unwrap_or_else(|| command.name.clone())
    ));

    if let Some(docs) = &command.docs {
        push_parameters(&mut lines, "Arguments", &docs.arguments);
        push_parameters(&mut lines, "Options", &docs.options);
        push_examples(&mut lines, &docs.examples);
        if !docs.output_notes.is_empty() {
            lines.push(String::new());
            lines.push("Output:".to_string());
            for note in &docs.output_notes {
                lines.push(format!("  - {note}"));
            }
        }
    }

    lines.join("\n")
}

pub(super) fn app_command_json(manifest: &AppManifest, command: &AppCommand) -> Value {
    json!({
        "app": manifest.id,
        "command": command,
    })
}

fn command_usage(command: &AppCommand) -> Option<String> {
    command
        .docs
        .as_ref()
        .map(|docs| docs.invocation.clone())
        .or_else(|| (!command.usage.is_empty()).then(|| command.usage.clone()))
}

fn push_parameters(lines: &mut Vec<String>, title: &str, parameters: &[AppCommandParameter]) {
    if parameters.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{title}:"));
    for parameter in parameters {
        let value_name = parameter
            .value_name
            .as_ref()
            .map(|value| format!(" <{value}>"))
            .unwrap_or_default();
        let required = if parameter.required {
            "required"
        } else {
            "optional"
        };
        lines.push(format!(
            "  - {}{} ({required}): {}",
            parameter.name, value_name, parameter.description
        ));
    }
}

fn push_examples(lines: &mut Vec<String>, examples: &[AppCommandExample]) {
    if examples.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("Examples:".to_string());
    for example in examples {
        lines.push(format!("  - {}: {}", example.title, example.command));
        lines.push(format!("    {}", example.description));
    }
}

pub(super) fn render_plan_with_fee_caps(plan: &ActionPlan, fee_caps: &[ApprovalFeeCap]) -> String {
    let mut lines = vec![
        format!("App: {} {}", plan.app_id, plan.app_version),
        format!("Chain: {}", plan.chain),
        "Action:".to_string(),
    ];
    for (step_index, step) in plan.steps.iter().enumerate() {
        lines.push(format!("  - {}", step.summary));
        if let Some(fee_cap) = fee_caps
            .iter()
            .find(|fee_cap| fee_cap.step_index == step_index)
        {
            lines.push(format!(
                "    Max network fee: {} wei",
                fee_cap.approved_max_total_fee_wei
            ));
            lines.push(format!(
                "    Approved gas limit: {}",
                fee_cap.approved_gas_limit
            ));
        }
    }
    if !plan.dynamic_contracts.is_empty() {
        lines.push("Invocation-scoped contracts:".to_string());
        for scope in &plan.dynamic_contracts {
            lines.push(format!("  - {} on {}", scope.contract, scope.chain));
        }
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
        render_plan_with_fee_caps(&record.plan, &record.fee_caps)
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

#[cfg(test)]
mod tests;
