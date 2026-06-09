// lint-long-file-override allow-max-lines=300
#[cfg(unix)]
use std::{fs::File, io::Write, os::fd::AsRawFd};

#[cfg(unix)]
use tempfile::NamedTempFile;

#[cfg(unix)]
use crate::commands::wallet_recovery::read_recovery_phrase_from_fd;

use super::fixtures::test_app_with_output;
use crate::{
    commands::wallet_recovery::{
        export_recovery_phrase_output, import_recovery_phrase_with_password,
        recovery_phrase_import_warning,
    },
    error::Error,
    keystore::{KeyStore, StoredWallet, decrypt_private_key, encrypt_private_key, wallet_address},
    output::OutputMode,
    runtime::{BeamApp, InvocationOverrides},
};

const PASSWORD: &str = "beam-password";
const PRIVATE_KEY: &str = "4f3edf983ac636a65a842ce7c78d9aa706d3b113bce036f6c4d1f06b2d1f6f9d";
const RECOVERY_PHRASE: &str = "execute want toward intact gloom farm head machine treat detect grit evoke honey sudden exclude orchard dad renew crucial this ready moral salmon pave";

#[cfg(unix)]
#[test]
fn reads_recovery_phrase_from_file_descriptor() {
    let recovery_phrase = "execute want toward intact gloom farm head machine treat detect grit evoke honey sudden exclude orchard dad renew crucial this ready moral salmon pave";
    let mut temp = NamedTempFile::new().expect("create temp recovery phrase file");
    write!(temp, "{recovery_phrase}").expect("write recovery phrase");

    let file = File::open(temp.path()).expect("open temp recovery phrase file");
    let actual = read_recovery_phrase_from_fd(file.as_raw_fd() as u32)
        .expect("read recovery phrase from file descriptor");

    assert_eq!(actual, recovery_phrase);
}

#[tokio::test]
async fn exports_default_wallet_recovery_phrase_with_semantic_warning() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let address = seed_encrypted_wallet(&app, "alice", PASSWORD).await;

    let output = export_recovery_phrase_output(&app, None, PASSWORD)
        .await
        .expect("export recovery phrase");

    assert!(output.default.contains("Recovery phrase for alice"));
    assert!(output.default.contains(&address));
    assert!(output.default.contains(RECOVERY_PHRASE));
    assert!(
        output
            .default
            .contains("direct BIP39 encoding of the raw EVM private key")
    );
    assert!(
        output
            .default
            .contains("not a MetaMask or HD-wallet seed phrase")
    );
    assert_eq!(output.compact.as_deref(), Some(RECOVERY_PHRASE));
    assert_eq!(output.value["address"].as_str(), Some(address.as_str()));
    assert_eq!(output.value["name"].as_str(), Some("alice"));
    assert_eq!(
        output.value["recovery_phrase"].as_str(),
        Some(RECOVERY_PHRASE)
    );
    assert_eq!(
        output.value["recovery_phrase_kind"].as_str(),
        Some("evm_private_key_bip39_entropy")
    );
    assert_eq!(output.value["hd_seed_phrase"].as_bool(), Some(false));
    assert!(
        output.value["warning"]
            .as_str()
            .expect("warning")
            .contains("not a MetaMask or HD-wallet seed phrase")
    );
}

#[tokio::test]
async fn export_recovery_phrase_rejects_wrong_password() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    seed_encrypted_wallet(&app, "alice", PASSWORD).await;

    let err = export_recovery_phrase_output(&app, None, "wrong-password")
        .await
        .expect_err("reject wrong password");

    assert!(matches!(err, Error::DecryptionFailed));
}

#[tokio::test]
async fn imports_recovery_phrase_with_password_and_persists_wallet() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let expected_private_key = private_key_bytes();
    let expected_address = format!(
        "{:#x}",
        wallet_address(&expected_private_key).expect("derive wallet address")
    );

    let output = import_recovery_phrase_with_password(
        &app,
        Some("restored".to_string()),
        RECOVERY_PHRASE,
        PASSWORD,
        Some(&expected_address),
    )
    .await
    .expect("import recovery phrase");

    assert_eq!(
        output.value["address"].as_str(),
        Some(expected_address.as_str())
    );
    assert_eq!(output.value["name"].as_str(), Some("restored"));

    let keystore = app.keystore_store.get().await;
    assert_eq!(keystore.wallets.len(), 1);
    let wallet = &keystore.wallets[0];
    assert_eq!(wallet.address, expected_address);
    assert_eq!(wallet.name, "restored");
    assert_eq!(
        decrypt_private_key(wallet, PASSWORD).expect("decrypt imported wallet"),
        expected_private_key
    );

    let config = app.config_store.get().await;
    assert_eq!(config.default_wallet.as_deref(), Some("restored"));

    let exported = export_recovery_phrase_output(&app, None, PASSWORD)
        .await
        .expect("export imported default wallet");
    assert_eq!(
        exported.value["recovery_phrase"].as_str(),
        Some(RECOVERY_PHRASE)
    );
}

#[tokio::test]
async fn import_recovery_phrase_rejects_unexpected_address_before_persisting() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;

    let err = import_recovery_phrase_with_password(
        &app,
        Some("restored".to_string()),
        RECOVERY_PHRASE,
        PASSWORD,
        Some("0x1111111111111111111111111111111111111111"),
    )
    .await
    .expect_err("reject unexpected derived address");

    match err {
        Error::RecoveryPhraseAddressMismatch { expected, derived } => {
            assert_eq!(expected, "0x1111111111111111111111111111111111111111");
            assert_ne!(derived, expected);
        }
        other => panic!("expected address mismatch, got {other:?}"),
    }
    assert!(app.keystore_store.get().await.wallets.is_empty());
}

#[tokio::test]
async fn duplicate_recovery_phrase_import_fails_before_password_validation() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let address = seed_encrypted_wallet(&app, "alice", PASSWORD).await;

    let err = import_recovery_phrase_with_password(
        &app,
        Some("duplicate".to_string()),
        RECOVERY_PHRASE,
        " \t ",
        Some(&address),
    )
    .await
    .expect_err("reject duplicate address before validating password");

    assert!(matches!(
        err,
        Error::WalletAddressAlreadyExists { address: duplicate } if duplicate == address
    ));
}

#[test]
fn recovery_phrase_import_warning_includes_derived_address_and_non_hd_warning() {
    let warning = recovery_phrase_import_warning("0x9a5ad45307715c47527a232b6c65978349c2411c");

    assert!(warning.contains("0x9a5ad45307715c47527a232b6c65978349c2411c"));
    assert!(warning.contains("direct BIP39 encoding of the raw EVM private key"));
    assert!(warning.contains("not a MetaMask or HD-wallet seed phrase"));
    assert!(warning.contains("Press Ctrl-C to cancel"));
}

async fn seed_encrypted_wallet(app: &BeamApp, name: &str, password: &str) -> String {
    let private_key = private_key_bytes();
    let encrypted_private_key =
        encrypt_private_key(&private_key, password).expect("encrypt private key");
    let address = format!(
        "{:#x}",
        wallet_address(&private_key).expect("derive wallet address")
    );

    app.keystore_store
        .set(KeyStore {
            wallets: vec![StoredWallet {
                address: address.clone(),
                encrypted_key: encrypted_private_key.encrypted_key,
                name: name.to_string(),
                salt: encrypted_private_key.salt,
                kdf: encrypted_private_key.kdf,
            }],
        })
        .await
        .expect("persist keystore");

    let default_wallet = name.to_string();
    app.config_store
        .update(move |config| config.default_wallet = Some(default_wallet.clone()))
        .await
        .expect("persist default wallet");

    address
}

fn private_key_bytes() -> Vec<u8> {
    hex::decode(PRIVATE_KEY).expect("decode private key")
}
