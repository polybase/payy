// lint-long-file-override allow-max-lines=290
use clap::Parser;
use tempfile::TempDir;

use crate::{
    cli::{Cli, Command, ProfileCommandKind, ProfileGrantKind, ProfilesAction},
    profiles::{
        Error as ProfileError, ledger,
        model::{
            AppActionIntent, AppGrant, CommandGrant, ProfilePolicy, ProfileRecord,
            PublicCommandKind, PublicSigningIntent, TransferIntent,
        },
        policy,
        session::{self, ProfileSession},
        store,
        wire::WireTransaction,
    },
};

const ADDRESS: &str = "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1";
const RECIPIENT: &str = "0xffcf8fdee72ac11b5c542428b35eef5769c409f0";

#[test]
fn parses_profiles_cli_surface() {
    let cli = Cli::try_parse_from([
        "beam",
        "--profile",
        "agent",
        "profiles",
        "grant",
        "agent",
        "command",
        "native-transfer",
        "--chain",
        "base",
        "--recipient",
        RECIPIENT,
        "--max-native",
        "100",
    ])
    .expect("parse profile grant");

    assert_eq!(cli.overrides().profile.as_deref(), Some("agent"));
    assert!(matches!(
        cli.command,
        Some(Command::Profiles {
            action: ProfilesAction::Grant(args)
        }) if matches!(
            &args.grant,
            ProfileGrantKind::Command(command)
                if command.command == ProfileCommandKind::NativeTransfer
                    && command.chain.as_deref() == Some("base")
        )
    ));
}

#[test]
fn profile_integrity_detects_tampering() {
    let key = b"test-integrity-key";
    let mut profile = profile_record();
    store::sign_profile(&mut profile, key).expect("sign profile");
    store::verify_profile(&profile, key).expect("verify profile");

    profile.wallet_address = RECIPIENT.to_string();
    let err = store::verify_profile(&profile, key).expect_err("reject tampered profile");

    assert!(matches!(err, ProfileError::ProfileIntegrityFailed { profile } if profile == "agent"));
}

#[test]
fn ledger_integrity_detects_tampering() {
    let key = b"test-integrity-key";
    let mut state = ledger::LedgerState {
        entries: vec![ledger::LedgerEntry {
            id: "led_test".to_string(),
            profile: "agent".to_string(),
            grant_id: "cmd_test".to_string(),
            created_at: 1,
            status: ledger::LedgerStatus::Signed,
            amount: "10".to_string(),
            asset: "native".to_string(),
            chain: "base".to_string(),
            gas: "21000".to_string(),
            app: None,
            command: Some("native-transfer".to_string()),
            plan_hash: None,
            approval_id: None,
            tx_hash: Some("0x01".to_string()),
        }],
        integrity: None,
    };
    ledger::sign_state(&mut state, key).expect("sign ledger");
    ledger::verify_state(&state, key).expect("verify ledger");

    state.entries[0].amount = "11".to_string();

    assert!(matches!(
        ledger::verify_state(&state, key),
        Err(ProfileError::LedgerIntegrityFailed)
    ));
}

#[test]
fn policy_allows_matching_native_transfer_and_denies_budget_overflow() {
    let mut profile = profile_record();
    profile.policy.command_grants.push(CommandGrant {
        id: "cmd_native".to_string(),
        command: PublicCommandKind::NativeTransfer,
        chain: Some("base".to_string()),
        token: None,
        recipient: Some(RECIPIENT.to_string()),
        target: None,
        selector: None,
        spender: None,
        max_native_value: Some("100".to_string()),
        max_token_amount: None,
        max_gas: Some("30000".to_string()),
        cumulative_budget: Some("100".to_string()),
        ttl_secs: None,
        allow_unlimited_approval: false,
    });
    let intent = PublicSigningIntent::NativeTransfer(TransferIntent {
        wallet: ADDRESS.to_string(),
        chain: "base".to_string(),
        recipient: RECIPIENT.to_string(),
        amount: "50".to_string(),
        asset: "native".to_string(),
    });
    let tx = native_tx("50");
    let ledger = ledger::LedgerState::default();

    let decision = policy::authorize(&profile, &ledger, &intent, &tx).expect("allow transfer");
    assert_eq!(decision.grant_id, "cmd_native");

    let mut spent = ledger::LedgerState::default();
    spent.entries.push(ledger::LedgerEntry {
        id: "led_existing".to_string(),
        profile: "agent".to_string(),
        grant_id: "cmd_native".to_string(),
        created_at: 1,
        status: ledger::LedgerStatus::Signed,
        amount: "60".to_string(),
        asset: "native".to_string(),
        chain: "base".to_string(),
        gas: "21000".to_string(),
        app: None,
        command: Some("native-transfer".to_string()),
        plan_hash: None,
        approval_id: None,
        tx_hash: None,
    });
    assert_eq!(
        ledger::spent_for_grant(&spent, "cmd_native", "native").to_string(),
        "60"
    );

    assert!(matches!(
        policy::authorize(&profile, &spent, &intent, &tx),
        Err(ProfileError::PolicyDenied { .. })
    ));
}

#[test]
fn policy_matches_app_plan_and_rejects_expired_plan() {
    let mut profile = profile_record();
    profile.policy.app_grants.push(AppGrant {
        id: "app_swap".to_string(),
        app_id: "uniswap".to_string(),
        registry_url: None,
        app_version: Some("1.0.0".to_string()),
        manifest_digest: Some("sha256:manifest".to_string()),
        module_digest: Some("sha256:wasm".to_string()),
        command: Some("swap".to_string()),
        wallet: Some(ADDRESS.to_string()),
        chain: Some("base".to_string()),
        plan_hash: Some("sha256:plan".to_string()),
        bindings: Vec::new(),
        constraints: Vec::new(),
        steps: Vec::new(),
        expires_at: None,
        max_gas: Some("100000".to_string()),
        cumulative_budget: None,
        allow_unlimited_approval: false,
    });
    let tx = native_tx("0");
    let intent = PublicSigningIntent::AppActionPlan(AppActionIntent {
        app_id: "uniswap".to_string(),
        app_version: "1.0.0".to_string(),
        registry_url: None,
        manifest_digest: "sha256:manifest".to_string(),
        module_digest: "sha256:wasm".to_string(),
        command: "swap".to_string(),
        wallet: ADDRESS.to_string(),
        chain: "base".to_string(),
        plan_hash: "sha256:plan".to_string(),
        expires_at: store::now() + 60,
        bindings: Vec::new(),
        constraints: Vec::new(),
        steps: Vec::new(),
        approval_id: Some("apr_test".to_string()),
    });

    policy::authorize(&profile, &ledger::LedgerState::default(), &intent, &tx)
        .expect("allow app plan");

    let PublicSigningIntent::AppActionPlan(mut expired) = intent else {
        panic!("expected app intent");
    };
    expired.expires_at = 1;

    assert!(matches!(
        policy::authorize(
            &profile,
            &ledger::LedgerState::default(),
            &PublicSigningIntent::AppActionPlan(expired),
            &tx,
        ),
        Err(ProfileError::PolicyDenied { .. })
    ));
}

#[tokio::test]
async fn session_active_rejects_expired_sessions() {
    let temp = TempDir::new().expect("create temp dir");
    session::upsert(
        temp.path(),
        ProfileSession {
            profile: "agent".to_string(),
            wallet_address: ADDRESS.to_string(),
            socket: temp.path().join("agent.sock"),
            token: "tok_test".to_string(),
            created_at: 1,
            expires_at: 1,
        },
    )
    .await
    .expect("persist session");

    assert!(matches!(
        session::active(temp.path(), "agent").await,
        Err(ProfileError::SessionExpired { profile }) if profile == "agent"
    ));
}

#[test]
fn profile_socket_path_stays_short_for_long_roots() {
    let temp = TempDir::new().expect("create temp dir");
    let long_root = temp.path().join("a".repeat(160));
    let socket = session::socket_path(&long_root, "agent");

    assert!(!socket.starts_with(&long_root));
    assert!(
        socket
            .file_name()
            .expect("socket filename")
            .to_string_lossy()
            .len()
            < 80
    );
}

fn profile_record() -> ProfileRecord {
    ProfileRecord {
        name: "agent".to_string(),
        wallet_address: ADDRESS.to_string(),
        wallet_selector: "alice".to_string(),
        created_at: 1,
        updated_at: 1,
        default_ttl_secs: 3600,
        policy: ProfilePolicy::default(),
        integrity: None,
    }
}

fn native_tx(value: &str) -> WireTransaction {
    WireTransaction {
        nonce: Some("0".to_string()),
        to: Some(RECIPIENT.to_string()),
        gas: "21000".to_string(),
        gas_price: Some("1".to_string()),
        value: value.to_string(),
        data: "0x".to_string(),
        chain_id: Some(8453),
        transaction_type: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}
