use serde_json::json;

use super::fixtures::test_app_with_output;
use crate::{
    commands::wallet_private_key::{
        export_private_key_output, export_private_key_output_for_selector,
    },
    error::Error,
    keystore::{KeyStore, StoredWallet, encrypt_private_key, wallet_address},
    output::OutputMode,
    runtime::InvocationOverrides,
};

const ALICE_PRIVATE_KEY: &str = "4f3edf983ac636a65a842ce7c78d9aa706d3b113bce036f6c4d1f06b2d1f6f9d";
const BOB_PRIVATE_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000002";
const PASSWORD: &str = "beam-password";

#[test]
fn export_private_key_output_includes_raw_key_only() {
    let wallet = stored_wallet("alice", ALICE_PRIVATE_KEY, PASSWORD);
    let output = export_private_key_output(&wallet, PASSWORD).expect("export private key");
    let private_key = private_key_hex(ALICE_PRIVATE_KEY);

    assert_eq!(output.compact.as_deref(), Some(private_key.as_str()));
    assert!(output.default.contains("Private key for alice"));
    assert!(output.default.contains(&wallet.address));
    assert!(output.default.contains(&private_key));
    assert!(
        output
            .default
            .contains("Anyone with it can control this wallet.")
    );
    assert_eq!(output.value["address"], json!(wallet.address));
    assert_eq!(output.value["name"], json!("alice"));
    assert_eq!(output.value["private_key"], json!(private_key));
    assert_eq!(
        output.value["warning"],
        json!("Store this key securely. Anyone with it can control this wallet.")
    );
    assert!(output.value.get("recovery_phrase").is_none());
}

#[test]
fn export_private_key_rejects_wrong_password() {
    let wallet = stored_wallet("alice", ALICE_PRIVATE_KEY, PASSWORD);
    let err =
        export_private_key_output(&wallet, "wrong-password").expect_err("reject wrong password");

    assert!(matches!(err, Error::DecryptionFailed));
}

#[tokio::test]
async fn export_private_key_resolves_default_and_explicit_wallets() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let alice = stored_wallet("alice", ALICE_PRIVATE_KEY, PASSWORD);
    let bob = stored_wallet("bob", BOB_PRIVATE_KEY, PASSWORD);
    app.keystore_store
        .set(KeyStore {
            wallets: vec![alice, bob],
        })
        .await
        .expect("persist wallets");
    app.config_store
        .update(|config| config.default_wallet = Some("bob".to_string()))
        .await
        .expect("persist default wallet");

    let default_output = export_private_key_output_for_selector(&app, None, PASSWORD)
        .await
        .expect("export default wallet private key");
    assert_eq!(
        default_output.value["private_key"],
        json!(private_key_hex(BOB_PRIVATE_KEY))
    );

    let explicit_output = export_private_key_output_for_selector(&app, Some("alice"), PASSWORD)
        .await
        .expect("export selected wallet private key");
    assert_eq!(
        explicit_output.value["private_key"],
        json!(private_key_hex(ALICE_PRIVATE_KEY))
    );

    let err = export_private_key_output_for_selector(&app, Some("missing"), PASSWORD)
        .await
        .expect_err("reject missing wallet selector");
    assert!(matches!(err, Error::WalletNotFound { selector } if selector == "missing"));
}

#[tokio::test]
async fn export_private_key_without_selector_uses_from_override_before_default() {
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            from: Some("alice".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;
    let alice = stored_wallet("alice", ALICE_PRIVATE_KEY, PASSWORD);
    let bob = stored_wallet("bob", BOB_PRIVATE_KEY, PASSWORD);
    app.keystore_store
        .set(KeyStore {
            wallets: vec![alice, bob],
        })
        .await
        .expect("persist wallets");
    app.config_store
        .update(|config| config.default_wallet = Some("bob".to_string()))
        .await
        .expect("persist default wallet");

    let output = export_private_key_output_for_selector(&app, None, PASSWORD)
        .await
        .expect("export overridden active wallet private key");

    assert_eq!(
        output.value["private_key"],
        json!(private_key_hex(ALICE_PRIVATE_KEY))
    );
}

fn stored_wallet(name: &str, private_key: &str, password: &str) -> StoredWallet {
    let secret_key = hex::decode(private_key).expect("decode private key");
    let encrypted_private_key =
        encrypt_private_key(&secret_key, password).expect("encrypt private key");

    StoredWallet {
        address: format!(
            "{:#x}",
            wallet_address(&secret_key).expect("derive wallet address")
        ),
        encrypted_key: encrypted_private_key.encrypted_key,
        name: name.to_string(),
        salt: encrypted_private_key.salt,
        kdf: encrypted_private_key.kdf,
    }
}

fn private_key_hex(private_key: &str) -> String {
    format!("0x{private_key}")
}
