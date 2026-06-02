// lint-long-file-override allow-max-lines=400
mod execution;
mod plans;
mod prompt;
mod render;

use std::fs;

use contextful::ResultContextExt;
use serde_json::json;

use crate::{
    apps::{
        Error as AppError,
        approvals::{ApprovalStore, ensure_approval_executable},
        model::{ActionPlan, AppManifest, ApprovalRecord},
        registry::{
            ensure_manifest_matches, fetch_index, fetch_manifest, fetch_module,
            registry_url_from_env, select_app, select_version,
        },
        runtime::validate_wasm_module,
        store::AppCache,
    },
    cli::{AppApprovalAction, AppInstallArgs, AppRemoveArgs, AppRunArgs, AppsAction},
    error::Result,
    output::CommandOutput,
    runtime::BeamApp,
    table::render_table,
};

use execution::execute_plan;
use plans::{plan_for_command, validate_plan_permissions};
use prompt::approve_interactively;
use render::{
    approval_json, manifest_json, permissions_json, render_app_help, render_approval,
    render_approval_created, render_execution, render_install_summary, render_manifest_info,
    render_permission_diff, render_permissions,
};

pub async fn run(app: &BeamApp, action: AppsAction) -> Result<()> {
    match action {
        AppsAction::Install(args) => install(app, args).await,
        AppsAction::List => list(app).await,
        AppsAction::Info { app: app_id } => info(app, &app_id).await,
        AppsAction::Update { app: app_id } => update(app, app_id.as_deref()).await,
        AppsAction::Remove(args) => remove(app, args).await,
        AppsAction::Permissions { app: app_id } => permissions(app, &app_id).await,
        AppsAction::Run(args) => run_app(app, args).await,
        AppsAction::Approvals { action } => approvals(app, action).await,
    }
}

pub async fn run_app(app: &BeamApp, args: AppRunArgs) -> Result<()> {
    let prepare = args.prepare || args.args.iter().any(|arg| arg == "--prepare");
    let no_prompt = args.no_prompt || args.args.iter().any(|arg| arg == "--no-prompt");
    let command_args = filtered_app_args(&args.args);
    let cache = AppCache::load(&app.paths.root).await?;
    let (installed, manifest) = cache.active_manifest(&args.app).await?;

    fs::create_dir_all(cache.data_dir(&args.app)).context("create beam app data directory")?;
    validate_wasm_module(
        &args.app,
        &manifest.wasm.entrypoint,
        &cache.module_path(&args.app, &installed.active_version),
    )?;

    let command = command_args
        .first()
        .cloned()
        .unwrap_or_else(|| "help".to_string());
    if command == "help" || args.args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return CommandOutput::new(render_app_help(&manifest), manifest_json(&manifest))
            .print(app.output_mode);
    }

    let plan = plan_for_command(app, &manifest, &installed, &command_args).await?;
    validate_plan_permissions(&manifest.permissions, &plan)?;
    let approval_required = plan_requires_approval(&plan);

    if prepare {
        let approvals = ApprovalStore::load(&app.paths.root).await?;
        let approval = approvals.create(plan).await?;
        return render_approval_created(&approval).print(app.output_mode);
    }

    if approval_required {
        if no_prompt {
            return Err(AppError::ApprovalRequired.into());
        }
        approve_interactively(&render::render_plan(&plan))?;
    }
    render_execution(&plan).print(app.output_mode)
}

fn plan_requires_approval(plan: &ActionPlan) -> bool {
    plan.steps
        .iter()
        .any(|step| step.kind == "erc20-approval" || step.kind == "transaction")
}

fn filtered_app_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|arg| arg.as_str() != "--prepare" && arg.as_str() != "--no-prompt")
        .cloned()
        .collect()
}

async fn install(app: &BeamApp, args: AppInstallArgs) -> Result<()> {
    let registry_url = registry_url_from_env();
    let index = fetch_index(&registry_url).await?;
    let registry_app = select_app(&index, &args.app)?;
    let version = select_version(registry_app, args.version.as_deref())?;
    let (manifest, manifest_bytes) = fetch_manifest(version).await?;
    ensure_manifest_matches(&args.app, version, &manifest)?;
    let module_bytes = fetch_module(version, &manifest).await?;
    let summary = render_install_summary(&manifest, version.module_sha256.as_str(), &registry_url);

    if args.dry_run {
        return CommandOutput::new(
            summary,
            json!({
                "app": manifest.id,
                "dry_run": true,
                "permissions": permissions_json(&manifest.permissions),
                "version": manifest.version,
            }),
        )
        .print(app.output_mode);
    }

    approve_interactively(&summary)?;
    let cache = AppCache::load(&app.paths.root).await?;
    cache
        .install(
            &manifest,
            &manifest_bytes,
            &module_bytes,
            &version.manifest_sha256,
            &version.module_sha256,
            &registry_url,
        )
        .await?;

    CommandOutput::new(
        format!("Installed {} {}", manifest.display_name, manifest.version),
        json!({
            "app": manifest.id,
            "installed": true,
            "permissions": permissions_json(&manifest.permissions),
            "version": manifest.version,
        }),
    )
    .print(app.output_mode)
}

async fn list(app: &BeamApp) -> Result<()> {
    let cache = AppCache::load(&app.paths.root).await?;
    let installed = cache.installed().await.installed;
    if installed.is_empty() {
        return CommandOutput::message("No Beam apps installed.").print(app.output_mode);
    }
    let rows = installed
        .iter()
        .map(|app| vec![app.id.clone(), app.active_version.clone()])
        .collect::<Vec<_>>();

    CommandOutput::new(
        render_table(&["App", "Version"], &rows),
        json!({ "apps": installed }),
    )
    .print(app.output_mode)
}

async fn info(app: &BeamApp, app_id: &str) -> Result<()> {
    let cache = AppCache::load(&app.paths.root).await?;
    let (_, manifest) = cache.active_manifest(app_id).await?;
    CommandOutput::new(render_manifest_info(&manifest), manifest_json(&manifest))
        .print(app.output_mode)
}

async fn update(app: &BeamApp, app_id: Option<&str>) -> Result<()> {
    let targets = match app_id {
        Some(app_id) => vec![app_id.to_string()],
        None => {
            let cache = AppCache::load(&app.paths.root).await?;
            cache
                .installed()
                .await
                .installed
                .into_iter()
                .map(|app| app.id)
                .collect::<Vec<_>>()
        }
    };
    if targets.is_empty() {
        return CommandOutput::message("No apps installed.").print(app.output_mode);
    }

    let mut updated = Vec::new();
    for target in targets {
        let manifest = update_one(app, &target).await?;
        updated.push(vec![manifest.id, manifest.version]);
    }

    CommandOutput::new(
        render_table(&["App", "Version"], &updated),
        json!({ "updated": updated }),
    )
    .print(app.output_mode)
}

async fn update_one(app: &BeamApp, app_id: &str) -> Result<AppManifest> {
    let registry_url = registry_url_from_env();
    let index = fetch_index(&registry_url).await?;
    let registry_app = select_app(&index, app_id)?;
    let version = select_version(registry_app, None)?;
    let (manifest, manifest_bytes) = fetch_manifest(version).await?;
    ensure_manifest_matches(app_id, version, &manifest)?;
    let module_bytes = fetch_module(version, &manifest).await?;
    let cache = AppCache::load(&app.paths.root).await?;
    let (_, current) = cache.active_manifest(app_id).await?;
    if current.permissions != manifest.permissions {
        approve_interactively(&render_permission_diff(&current, &manifest))?;
    }
    cache
        .install(
            &manifest,
            &manifest_bytes,
            &module_bytes,
            &version.manifest_sha256,
            &version.module_sha256,
            &registry_url,
        )
        .await?;

    Ok(manifest)
}

async fn remove(app: &BeamApp, args: AppRemoveArgs) -> Result<()> {
    AppCache::load(&app.paths.root)
        .await?
        .remove(&args.app, args.purge_data)
        .await?;
    CommandOutput::new(
        format!("Removed {}", args.app),
        json!({ "app": args.app, "purged_data": args.purge_data, "removed": true }),
    )
    .print(app.output_mode)
}

async fn permissions(app: &BeamApp, app_id: &str) -> Result<()> {
    let cache = AppCache::load(&app.paths.root).await?;
    let (_, manifest) = cache.active_manifest(app_id).await?;
    CommandOutput::new(
        render_permissions(&manifest.permissions),
        permissions_json(&manifest.permissions),
    )
    .print(app.output_mode)
}

async fn approvals(app: &BeamApp, action: AppApprovalAction) -> Result<()> {
    let store = ApprovalStore::load(&app.paths.root).await?;
    match action {
        AppApprovalAction::List => list_approvals(app, store.list().await),
        AppApprovalAction::Show { approval_id } => {
            let approval = store.find(&approval_id).await?;
            CommandOutput::new(render_approval(&approval), approval_json(&approval))
                .print(app.output_mode)
        }
        AppApprovalAction::Approve {
            approval_id,
            execute,
        } => {
            let approval = store.find(&approval_id).await?;
            if execute {
                ensure_approval_executable(&approval)?;
                ensure_approval_matches_active(app, &approval).await?;
                let output = execute_plan(app, &approval.plan).await?;
                store.mark_executed(&approval_id).await?;
                return output.print(app.output_mode);
            }
            let approval = store.approve(&approval_id).await?;
            CommandOutput::new(
                format!("Approved {}", approval.id),
                approval_json(&approval),
            )
            .print(app.output_mode)
        }
        AppApprovalAction::Reject { approval_id } => {
            let approval = store.reject(&approval_id).await?;
            CommandOutput::new(
                format!("Rejected {}", approval.id),
                approval_json(&approval),
            )
            .print(app.output_mode)
        }
    }
}

fn list_approvals(app: &BeamApp, approvals: Vec<ApprovalRecord>) -> Result<()> {
    let rows = approvals
        .iter()
        .map(|approval| {
            vec![
                approval.id.clone(),
                format!("{:?}", approval.status),
                approval.plan.command.clone(),
            ]
        })
        .collect::<Vec<_>>();
    CommandOutput::new(
        render_table(&["Approval", "Status", "Command"], &rows),
        json!({ "approvals": approvals }),
    )
    .print(app.output_mode)
}

async fn ensure_approval_matches_active(app: &BeamApp, approval: &ApprovalRecord) -> Result<()> {
    let cache = AppCache::load(&app.paths.root).await?;
    let (installed, _) = cache.active_manifest(&approval.plan.app_id).await?;
    let artifact_matches = installed.active_version == approval.plan.app_version
        && installed.manifest_sha256 == approval.plan.manifest_sha256
        && installed.module_sha256 == approval.plan.wasm_sha256;
    if !artifact_matches {
        return Err(AppError::ApprovalArtifactChanged {
            approval_id: approval.id.clone(),
        }
        .into());
    }

    let active_chain = app.active_chain().await?;
    if active_chain.entry.key != approval.plan.chain {
        return Err(AppError::ApprovalContextChanged {
            actual: active_chain.entry.key,
            approval_id: approval.id.clone(),
            expected: approval.plan.chain.clone(),
            field: "chain".to_string(),
        }
        .into());
    }

    if let Some(expected_wallet) = approval.plan.wallet.as_ref() {
        let actual_wallet = format!("{:#x}", app.active_address().await?);
        if !actual_wallet.eq_ignore_ascii_case(expected_wallet) {
            return Err(AppError::ApprovalContextChanged {
                actual: actual_wallet,
                approval_id: approval.id.clone(),
                expected: expected_wallet.clone(),
                field: "wallet".to_string(),
            }
            .into());
        }
    }

    Ok(())
}
