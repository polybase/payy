// lint-long-file-override allow-max-lines=295
mod grants;
mod session_helpers;

use serde_json::json;

use crate::{
    cli::{
        ProfileCreateArgs, ProfileGrantArgs, ProfileGrantKind, ProfileRevokeArgs,
        ProfileUnlockArgs, ProfilesAction,
    },
    error::Result,
    keystore::{decrypt_private_key, prompt_existing_password},
    output::CommandOutput,
    profiles::{
        Error as ProfileError, ledger,
        model::{ProfilePolicy, ProfileRecord},
        session::{self, ProfileSession},
        signer,
        store::{self, integrity_key, parse_duration_secs, validate_profile_name},
    },
    runtime::BeamApp,
    table::render_table,
};

use grants::{app_grant, command_grant};
use session_helpers::{shell_escape, spawn_daemon, wait_for_daemon};

const DEFAULT_PROFILE_TTL_SECS: u64 = 60 * 60;

pub async fn run(app: &BeamApp, action: ProfilesAction) -> Result<()> {
    match action {
        ProfilesAction::Create(args) => create(app, args).await,
        ProfilesAction::List => list(app).await,
        ProfilesAction::Show { profile } => show(app, &profile).await,
        ProfilesAction::Grant(args) => grant(app, *args).await,
        ProfilesAction::Revoke(args) => revoke(app, args).await,
        ProfilesAction::Remove { profile } => remove(app, &profile).await,
        ProfilesAction::Ledger { profile } => ledger(app, &profile).await,
        ProfilesAction::Unlock(args) => unlock(app, args).await,
        ProfilesAction::Lock { profile } => lock(app, &profile).await,
        ProfilesAction::Sessions => sessions(app).await,
    }
}

async fn create(app: &BeamApp, args: ProfileCreateArgs) -> Result<()> {
    validate_profile_name(&args.profile)?;
    let wallet = app.resolve_wallet(&args.from).await?;
    let password = prompt_existing_password()?;
    if password.is_empty() {
        return Err(ProfileError::EmptyPasswordWallet.into());
    }
    let secret_key = decrypt_private_key(&wallet, &password)?;
    let now = store::now();
    let profile = ProfileRecord {
        name: args.profile.clone(),
        wallet_address: wallet.address.clone(),
        wallet_selector: args.from,
        created_at: now,
        updated_at: now,
        default_ttl_secs: DEFAULT_PROFILE_TTL_SECS,
        policy: ProfilePolicy::default(),
        integrity: None,
    };
    store::insert(
        &app.paths.root,
        profile.clone(),
        &integrity_key(&secret_key),
    )
    .await?;
    CommandOutput::new(
        format!("Created profile {}", profile.name),
        json!({ "profile": profile.name, "wallet": profile.wallet_address }),
    )
    .print(app.output_mode)
}

async fn list(app: &BeamApp) -> Result<()> {
    let profiles = store::list(&app.paths.root).await?;
    let rows = profiles
        .iter()
        .map(|profile| vec![profile.name.clone(), profile.wallet_address.clone()])
        .collect::<Vec<_>>();
    CommandOutput::new(
        render_table(&["Profile", "Wallet"], &rows),
        json!({ "profiles": profiles }),
    )
    .print(app.output_mode)
}

async fn show(app: &BeamApp, profile: &str) -> Result<()> {
    let profile = store::find(&app.paths.root, profile).await?;
    CommandOutput::new(render_profile(&profile), json!({ "profile": profile }))
        .print(app.output_mode)
}

async fn grant(app: &BeamApp, args: ProfileGrantArgs) -> Result<()> {
    let secret_key = profile_secret(app, &args.profile).await?;
    let key = integrity_key(&secret_key);
    let output = match args.grant {
        ProfileGrantKind::Command(grant_args) => {
            let grant = command_grant(grant_args)?;
            let grant_id = grant.id.clone();
            store::update_verified(&app.paths.root, &args.profile, &key, |profile| {
                profile.policy.command_grants.push(grant);
                Ok(())
            })
            .await?;
            json!({ "grant_id": grant_id, "kind": "command" })
        }
        ProfileGrantKind::App(grant_args) => {
            let grant = app_grant(app, grant_args).await?;
            let grant_id = grant.id.clone();
            store::update_verified(&app.paths.root, &args.profile, &key, |profile| {
                profile.policy.app_grants.push(grant);
                Ok(())
            })
            .await?;
            json!({ "grant_id": grant_id, "kind": "app" })
        }
    };
    CommandOutput::new("Granted profile capability".to_string(), output).print(app.output_mode)
}

async fn revoke(app: &BeamApp, args: ProfileRevokeArgs) -> Result<()> {
    let secret_key = profile_secret(app, &args.profile).await?;
    let key = integrity_key(&secret_key);
    store::update_verified(&app.paths.root, &args.profile, &key, |profile| {
        let before = profile.policy.command_grants.len() + profile.policy.app_grants.len();
        profile
            .policy
            .command_grants
            .retain(|grant| grant.id != args.grant_id);
        profile
            .policy
            .app_grants
            .retain(|grant| grant.id != args.grant_id);
        let after = profile.policy.command_grants.len() + profile.policy.app_grants.len();
        if before == after {
            return Err(ProfileError::GrantNotFound {
                grant_id: args.grant_id.clone(),
            });
        }
        Ok(())
    })
    .await?;
    CommandOutput::new(
        format!("Revoked {}", args.grant_id),
        json!({ "grant_id": args.grant_id, "revoked": true }),
    )
    .print(app.output_mode)
}

async fn remove(app: &BeamApp, profile: &str) -> Result<()> {
    let secret_key = profile_secret(app, profile).await?;
    store::remove_verified(&app.paths.root, profile, &integrity_key(&secret_key)).await?;
    let _ = session::remove(&app.paths.root, profile).await;
    CommandOutput::new(
        format!("Removed profile {profile}"),
        json!({ "profile": profile, "removed": true }),
    )
    .print(app.output_mode)
}

async fn ledger(app: &BeamApp, profile: &str) -> Result<()> {
    let state = ledger::load(&app.paths.root).await?.get().await;
    let entries = state
        .entries
        .into_iter()
        .filter(|entry| entry.profile == profile)
        .collect::<Vec<_>>();
    let rows = entries
        .iter()
        .map(|entry| {
            vec![
                entry.id.clone(),
                format!("{:?}", entry.status),
                entry.chain.clone(),
                entry.amount.clone(),
                entry.tx_hash.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    CommandOutput::new(
        render_table(&["Entry", "Status", "Chain", "Amount", "Tx"], &rows),
        json!({ "entries": entries }),
    )
    .print(app.output_mode)
}

async fn unlock(app: &BeamApp, args: ProfileUnlockArgs) -> Result<()> {
    let profile = store::find(&app.paths.root, &args.profile).await?;
    let secret_key = profile_secret(app, &args.profile).await?;
    store::verify_profile(&profile, &integrity_key(&secret_key))?;
    let ttl = args
        .ttl
        .as_deref()
        .map(parse_duration_secs)
        .transpose()?
        .unwrap_or(profile.default_ttl_secs);
    let token = session::new_token();
    let expires_at = store::now().saturating_add(ttl);
    let socket = session::socket_path(&app.paths.root, &profile.name);
    spawn_daemon(app, &profile.name, &socket, &token, expires_at, &secret_key)?;
    let session = ProfileSession {
        profile: profile.name.clone(),
        wallet_address: profile.wallet_address.clone(),
        socket,
        token,
        created_at: store::now(),
        expires_at,
    };
    wait_for_daemon(&session).await?;
    session::upsert(&app.paths.root, session.clone()).await?;
    let message = if args.print_env {
        format!(
            "export BEAM_PROFILE={}\nexport BEAM_PROFILE_TOKEN={}",
            shell_escape(&session.profile),
            shell_escape(&session.token)
        )
    } else {
        format!(
            "Unlocked profile {} until {}",
            session.profile, session.expires_at
        )
    };
    CommandOutput::new(
        message,
        json!({ "expires_at": session.expires_at, "profile": session.profile }),
    )
    .print(app.output_mode)
}

async fn lock(app: &BeamApp, profile: &str) -> Result<()> {
    if let Ok(session) = session::active(&app.paths.root, profile).await {
        let _ = signer::lock(&session).await;
    }
    session::remove(&app.paths.root, profile).await?;
    CommandOutput::new(
        format!("Locked profile {profile}"),
        json!({ "locked": true, "profile": profile }),
    )
    .print(app.output_mode)
}

async fn sessions(app: &BeamApp) -> Result<()> {
    let sessions = session::list(&app.paths.root).await?;
    let rows = sessions
        .iter()
        .map(|session| {
            vec![
                session.profile.clone(),
                session.wallet_address.clone(),
                session.expires_at.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    CommandOutput::new(
        render_table(&["Profile", "Wallet", "Expires"], &rows),
        json!({ "sessions": sessions }),
    )
    .print(app.output_mode)
}

async fn profile_secret(app: &BeamApp, profile: &str) -> Result<Vec<u8>> {
    let profile = store::find(&app.paths.root, profile).await?;
    let wallet = app.resolve_wallet(&profile.wallet_address).await?;
    let password = prompt_existing_password()?;
    if password.is_empty() {
        return Err(ProfileError::EmptyPasswordWallet.into());
    }
    decrypt_private_key(&wallet, &password)
}

fn render_profile(profile: &ProfileRecord) -> String {
    let rows = vec![
        vec!["profile".to_string(), profile.name.clone()],
        vec!["wallet".to_string(), profile.wallet_address.clone()],
        vec![
            "command grants".to_string(),
            profile.policy.command_grants.len().to_string(),
        ],
        vec![
            "app grants".to_string(),
            profile.policy.app_grants.len().to_string(),
        ],
    ];
    render_table(&["Field", "Value"], &rows)
}
