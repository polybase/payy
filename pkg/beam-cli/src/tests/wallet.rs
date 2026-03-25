// lint-long-file-override allow-max-lines=300
#[cfg(unix)]
use std::{fs::File, io::Write, os::fd::AsRawFd};

#[cfg(unix)]
use tempfile::NamedTempFile;

use super::{
    ens::{set_ethereum_rpc, spawn_ens_rpc_server},
    fixtures::test_app_with_output,
};
#[cfg(unix)]
use crate::commands::wallet::read_private_key_from_fd;
use crate::{
    cli::{PrivateKeySourceArgs, WalletAction},
    commands::wallet::{normalize_wallet_name, rename_wallet, run as run_wallet_command},
    error::Error,
    keystore::{KeyStore, StoredKdf, StoredWallet, validate_wallet_name},
    output::OutputMode,
    runtime::{BeamApp, InvocationOverrides},
};

const ALICE_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
const BOB_ADDRESS: &str = "0x2222222222222222222222222222222222222222";

async fn seed_wallets(app: &BeamApp, default_wallet: Option<&str>, wallets: &[(&str, &str)]) {
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

    let default_wallet = default_wallet.map(ToString::to_string);
    app.config_store
        .update(move |config| config.default_wallet = default_wallet.clone())
        .await
        .expect("persist default wallet");
}

#[tokio::test]
async fn renames_wallet_and_updates_default_wallet() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    seed_wallets(
        &app,
        Some("alice"),
        &[("alice", ALICE_ADDRESS), ("bob", BOB_ADDRESS)],
    )
    .await;

    rename_wallet(&app, "alice", "primary")
        .await
        .expect("rename wallet");

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets[0].name, "primary");
    assert_eq!(keystore.wallets[0].address, ALICE_ADDRESS);
    assert_eq!(keystore.wallets[1].name, "bob");

    let config = app.config_store.get().await;
    assert_eq!(config.default_wallet.as_deref(), Some("primary"));
}

#[tokio::test]
async fn rename_rejects_existing_wallet_name() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    seed_wallets(
        &app,
        Some("alice"),
        &[("alice", ALICE_ADDRESS), ("bob", BOB_ADDRESS)],
    )
    .await;

    let err = rename_wallet(&app, "alice", "bob")
        .await
        .expect_err("reject duplicate wallet name");
    assert!(matches!(err, Error::WalletNameAlreadyExists { name } if name == "bob"));

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets[0].name, "alice");
    assert_eq!(keystore.wallets[1].name, "bob");

    let config = app.config_store.get().await;
    assert_eq!(config.default_wallet.as_deref(), Some("alice"));
}

#[tokio::test]
async fn rename_rejects_wallet_names_with_address_prefix() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    seed_wallets(&app, Some("alice"), &[("alice", ALICE_ADDRESS)]).await;

    let err = rename_wallet(&app, "alice", "0xprimary")
        .await
        .expect_err("reject address-like wallet names");

    assert!(matches!(
        err,
        Error::WalletNameStartsWithAddressPrefix { name } if name == "0xprimary"
    ));

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets[0].name, "alice");
}

#[tokio::test]
async fn rename_trims_wallet_names_before_persisting() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    seed_wallets(&app, Some("alice"), &[("alice", ALICE_ADDRESS)]).await;

    rename_wallet(&app, "alice", " primary ")
        .await
        .expect("rename wallet with trimmed name");

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets[0].name, "primary");

    let config = app.config_store.get().await;
    assert_eq!(config.default_wallet.as_deref(), Some("primary"));
}

#[tokio::test]
async fn rename_rejects_blank_wallet_names_after_trimming() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    seed_wallets(&app, Some("alice"), &[("alice", ALICE_ADDRESS)]).await;

    let err = rename_wallet(&app, "alice", " \t ")
        .await
        .expect_err("reject blank wallet names after trimming");

    assert!(matches!(err, Error::WalletNameBlank));

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets[0].name, "alice");
}

#[test]
fn normalize_wallet_name_trims_surrounding_whitespace() {
    let name = normalize_wallet_name(" alice ").expect("normalize wallet name");
    assert_eq!(name, "alice");
}

#[test]
fn normalize_wallet_name_rejects_blank_input() {
    let err = normalize_wallet_name("  ").expect_err("reject blank wallet name");
    assert!(matches!(err, Error::WalletNameBlank));
}

#[test]
fn normalize_wallet_name_sanitizes_control_characters() {
    let name = normalize_wallet_name(" ali\nce\x1b ").expect("normalize wallet name");
    assert_eq!(name, "ali ce?");
}

#[test]
fn validate_wallet_name_rejects_trimmed_duplicates() {
    let wallets = [StoredWallet {
        address: ALICE_ADDRESS.to_string(),
        encrypted_key: "encrypted-key".to_string(),
        name: "alice".to_string(),
        salt: "salt".to_string(),
        kdf: StoredKdf::default(),
    }];

    let err = validate_wallet_name(&wallets, " alice ", None)
        .expect_err("reject duplicate wallet names after trimming");
    assert!(matches!(err, Error::WalletNameAlreadyExists { name } if name == "alice"));
}

#[tokio::test]
async fn rename_allows_case_only_wallet_name_changes() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    seed_wallets(&app, Some("alice"), &[("alice", ALICE_ADDRESS)]).await;

    rename_wallet(&app, "alice", "Alice")
        .await
        .expect("rename wallet with case-only change");

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets[0].name, "Alice");

    let config = app.config_store.get().await;
    assert_eq!(config.default_wallet.as_deref(), Some("Alice"));
}

#[tokio::test]
async fn rename_accepts_matching_ens_wallet_name() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;
    seed_wallets(&app, Some("alice"), &[("alice", ALICE_ADDRESS)]).await;

    rename_wallet(&app, "alice", "alice.eth")
        .await
        .expect("rename wallet to matching ens name");
    server.abort();

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets[0].name, "alice.eth");

    let config = app.config_store.get().await;
    assert_eq!(config.default_wallet.as_deref(), Some("alice.eth"));
}

#[tokio::test]
async fn rename_rejects_mismatched_ens_wallet_name() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", BOB_ADDRESS).await;
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;
    seed_wallets(&app, Some("alice"), &[("alice", ALICE_ADDRESS)]).await;

    let err = rename_wallet(&app, "alice", "alice.eth")
        .await
        .expect_err("reject ens name that points elsewhere");
    server.abort();

    assert!(matches!(
        err,
        Error::WalletNameEnsAddressMismatch { address, name }
            if name == "alice.eth" && address == ALICE_ADDRESS
    ));

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets[0].name, "alice");
}

#[tokio::test]
async fn wallet_use_accepts_ens_selector() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", ALICE_ADDRESS).await;
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    set_ethereum_rpc(&app, &rpc_url).await;
    seed_wallets(&app, None, &[("primary", ALICE_ADDRESS)]).await;

    run_wallet_command(
        &app,
        WalletAction::Use {
            name: "alice.eth".to_string(),
        },
    )
    .await
    .expect("use wallet via ens selector");
    server.abort();

    let config = app.config_store.get().await;
    assert_eq!(config.default_wallet.as_deref(), Some("primary"));
}

#[cfg(unix)]
#[test]
fn reads_private_key_from_file_descriptor() {
    let mut temp = NamedTempFile::new().expect("create temp private key file");
    write!(temp, "0x0123").expect("write private key");

    let file = File::open(temp.path()).expect("open temp private key file");
    let private_key = read_private_key_from_fd(file.as_raw_fd() as u32)
        .expect("read private key from file descriptor");

    assert_eq!(private_key, "0x0123");
}

#[cfg(unix)]
#[tokio::test]
async fn wallet_address_rejects_malformed_private_key_hex() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let mut temp = NamedTempFile::new().expect("create temp private key file");
    write!(temp, "not-hex").expect("write malformed private key");

    let file = File::open(temp.path()).expect("open temp private key file");
    let err = run_wallet_command(
        &app,
        WalletAction::Address {
            private_key_source: PrivateKeySourceArgs {
                private_key_stdin: false,
                private_key_fd: Some(file.as_raw_fd() as u32),
            },
        },
    )
    .await
    .expect_err("reject malformed private key hex");

    assert!(matches!(err, Error::InvalidPrivateKey));
}
