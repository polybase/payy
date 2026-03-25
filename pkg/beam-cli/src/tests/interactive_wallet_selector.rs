use super::{
    ens::{set_ethereum_rpc, spawn_ens_rpc_server},
    fixtures::test_app,
};
use crate::{
    commands::interactive::{canonicalize_startup_wallet_override, handle_repl_command, prompt},
    error::Error,
    keystore::{KeyStore, StoredKdf, StoredWallet},
    runtime::{BeamApp, InvocationOverrides},
};

const ALICE_ADDRESS: &str = "0x1111111111111111111111111111111111111111";

async fn seed_wallets(app: &BeamApp, wallets: &[(&str, &str)]) {
    app.keystore_store
        .set(KeyStore {
            wallets: wallets
                .iter()
                .map(|(name, address)| StoredWallet {
                    address: (*address).to_string(),
                    encrypted_key: "encrypted-key".to_string(),
                    name: (*name).to_string(),
                    salt: "salt".to_string(),
                    kdf: StoredKdf::default(),
                })
                .collect(),
        })
        .await
        .expect("persist keystore");
}

#[tokio::test]
async fn canonical_wallet_selector_preserves_wallet_name_or_canonical_address() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    seed_wallets(&app, &[("Alice", ALICE_ADDRESS)]).await;

    let by_name = app
        .canonical_wallet_selector(Some("alice"))
        .await
        .expect("canonicalize wallet name");
    let by_address = app
        .canonical_wallet_selector(Some(&ALICE_ADDRESS.to_ascii_uppercase()))
        .await
        .expect("canonicalize wallet address");
    let raw_address = app
        .canonical_wallet_selector(Some("0x2222222222222222222222222222222222222222"))
        .await
        .expect("canonicalize raw address");

    assert_eq!(by_name.as_deref(), Some("Alice"));
    assert_eq!(by_address.as_deref(), Some("Alice"));
    assert_eq!(
        raw_address.as_deref(),
        Some("0x2222222222222222222222222222222222222222")
    );
}

#[tokio::test]
async fn canonical_wallet_selector_resolves_ens_to_a_canonical_address() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;

    let selector = app
        .canonical_wallet_selector(Some("Alice.ETH"))
        .await
        .expect("canonicalize ens selector");
    server.abort();

    assert_eq!(selector.as_deref(), Some(ALICE_ADDRESS));
}

#[tokio::test]
async fn startup_ens_override_is_canonicalized_once_for_prompt_rendering() {
    let (rpc_url, calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some("Alice.ETH".to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    set_ethereum_rpc(&app, &rpc_url).await;
    seed_wallets(&app, &[("Primary", ALICE_ADDRESS)]).await;

    let mut overrides = app.overrides.clone();
    canonicalize_startup_wallet_override(&app, &mut overrides)
        .await
        .expect("canonicalize startup ens selector");

    let ens_calls_after_startup = calls.lock().expect("lock ens calls").len();
    let session = BeamApp {
        overrides,
        ..app.clone()
    };
    prompt(&session)
        .await
        .expect("render prompt after startup canonicalization");
    prompt(&session)
        .await
        .expect("render prompt again without ens lookup");
    let ens_calls_after_prompts = calls.lock().expect("lock ens calls").len();
    server.abort();

    assert_eq!(session.overrides.from.as_deref(), Some("Primary"));
    assert_eq!(ens_calls_after_prompts, ens_calls_after_startup);
}

#[tokio::test]
async fn interactive_prompt_surfaces_invalid_wallet_override() {
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some("alcie".to_string()),
        ..InvocationOverrides::default()
    })
    .await;

    let err = match prompt(&app).await {
        Ok(_) => panic!("expected invalid prompt selector"),
        Err(err) => err,
    };

    assert!(matches!(err, Error::WalletNotFound { selector } if selector == "alcie"));
}

#[tokio::test]
async fn wallet_shortcut_rejects_unknown_wallet_without_mutating_state() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let mut overrides = InvocationOverrides {
        from: Some("Alice".to_string()),
        ..InvocationOverrides::default()
    };
    seed_wallets(&app, &[("Alice", ALICE_ADDRESS)]).await;

    let err = handle_repl_command(
        &app,
        &mut overrides,
        &["wallets".to_string(), "alcie".to_string()],
    )
    .await
    .expect_err("reject invalid wallet selector");

    assert!(matches!(err, Error::WalletNotFound { selector } if selector == "alcie"));
    assert_eq!(overrides.from.as_deref(), Some("Alice"));
}
