use contextful::ResultContextExt;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    apps::{
        approvals::{ApprovalStore, plan_hash},
        model::ActionPlan,
    },
    cli::{ProfileAppGrantArgs, ProfileCommandGrantArgs, ProfileCommandKind},
    profiles::{
        self, Error as ProfileError,
        model::{ActionGrantBinding, ActionGrantStep, AppGrant, CommandGrant, PublicCommandKind},
        store::{self, parse_duration_secs},
    },
    runtime::BeamApp,
};

pub async fn app_grant(app: &BeamApp, args: ProfileAppGrantArgs) -> profiles::Result<AppGrant> {
    let plan = grant_plan(app, &args).await?;
    let ttl_secs = args.ttl.as_deref().map(parse_duration_secs).transpose()?;
    let expires_at = ttl_secs.map(|ttl| store::now().saturating_add(ttl));
    let seed = serde_json::to_value((&args.app, &args.command, &args.plan_hash))
        .context("encode app grant seed")?;
    let mut grant = AppGrant {
        id: grant_id("app", &seed),
        app_id: args.app.clone(),
        registry_url: args.registry_url.clone(),
        app_version: args.version.clone(),
        manifest_digest: args.manifest_digest.clone(),
        module_digest: args.module_digest.clone(),
        command: args.command.clone(),
        wallet: args.wallet.clone(),
        chain: args.chain.clone(),
        plan_hash: args.plan_hash.clone(),
        bindings: Vec::new(),
        constraints: Vec::new(),
        steps: Vec::new(),
        expires_at,
        max_gas: args.max_gas,
        cumulative_budget: args.budget,
        allow_unlimited_approval: args.allow_unlimited_approval,
    };
    if let Some((plan, hash)) = plan {
        apply_plan(&mut grant, plan, hash);
    }
    Ok(grant)
}

pub fn command_grant(args: ProfileCommandGrantArgs) -> profiles::Result<CommandGrant> {
    let ttl_secs = args.ttl.as_deref().map(parse_duration_secs).transpose()?;
    let command = match args.command {
        ProfileCommandKind::NativeTransfer => PublicCommandKind::NativeTransfer,
        ProfileCommandKind::Erc20Transfer => PublicCommandKind::Erc20Transfer,
        ProfileCommandKind::Erc20Approval => PublicCommandKind::Erc20Approval,
        ProfileCommandKind::ContractTransaction => PublicCommandKind::ContractTransaction,
        ProfileCommandKind::FetchPayment => PublicCommandKind::FetchPayment,
    };
    Ok(CommandGrant {
        id: grant_id("cmd", &command_seed(&args)),
        command,
        chain: args.chain,
        token: args.token,
        recipient: args.recipient,
        target: args.target,
        selector: args.selector,
        spender: args.spender,
        max_native_value: args.max_native,
        max_token_amount: args.max_token,
        max_gas: args.max_gas,
        cumulative_budget: args.budget,
        ttl_secs,
        allow_unlimited_approval: args.allow_unlimited_approval,
    })
}

async fn grant_plan(
    app: &BeamApp,
    args: &ProfileAppGrantArgs,
) -> profiles::Result<Option<(ActionPlan, String)>> {
    if let Some(approval_id) = args.approval_id.as_deref() {
        let approval = ApprovalStore::load(&app.paths.root)
            .await
            .map_err(profile_policy_error)?
            .find(approval_id)
            .await
            .map_err(profile_policy_error)?;
        return Ok(Some((approval.plan, approval.plan_hash)));
    }
    if let Some(path) = args.plan_json.as_ref() {
        let bytes = std::fs::read(path).context("read profile app action plan json")?;
        let plan =
            serde_json::from_slice::<ActionPlan>(&bytes).context("decode profile action plan")?;
        let hash = plan_hash(&plan).map_err(profile_policy_error)?;
        return Ok(Some((plan, hash)));
    }
    Ok(None)
}

fn apply_plan(grant: &mut AppGrant, plan: ActionPlan, hash: String) {
    grant.app_id = plan.app_id.clone();
    grant.app_version = Some(plan.app_version.clone());
    grant.manifest_digest = Some(plan.manifest_sha256.clone());
    grant.module_digest = Some(plan.wasm_sha256.clone());
    grant.command = Some(plan.command.clone());
    grant.wallet = plan.wallet.clone();
    grant.chain = Some(plan.chain.clone());
    grant.plan_hash = Some(hash);
    grant.bindings = plan
        .bindings
        .into_iter()
        .map(|binding| ActionGrantBinding {
            key: binding.key,
            value: binding.value,
        })
        .collect();
    grant.constraints = plan.constraints;
    grant.steps = plan
        .steps
        .into_iter()
        .map(|step| ActionGrantStep {
            kind: step.kind,
            target: step.target,
            selector: step.selector,
            spender: step.spender,
            value: step.value,
            metadata: step.metadata,
        })
        .collect();
    grant.expires_at = Some(grant.expires_at.map_or(plan.expires_at, |expires_at| {
        expires_at.min(plan.expires_at)
    }));
}

fn command_seed(args: &ProfileCommandGrantArgs) -> serde_json::Value {
    json!({
        "budget": args.budget.clone(),
        "chain": args.chain.clone(),
        "command": format!("{:?}", args.command),
        "max_gas": args.max_gas.clone(),
        "max_native": args.max_native.clone(),
        "max_token": args.max_token.clone(),
        "recipient": args.recipient.clone(),
        "selector": args.selector.clone(),
        "spender": args.spender.clone(),
        "target": args.target.clone(),
        "token": args.token.clone(),
        "ttl": args.ttl.clone(),
    })
}

fn grant_id(prefix: &str, value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    format!("{prefix}_{}", hex::encode(&digest[..8]))
}

fn profile_policy_error(err: impl std::error::Error) -> ProfileError {
    ProfileError::PolicyDenied {
        reason: err.to_string(),
    }
}
