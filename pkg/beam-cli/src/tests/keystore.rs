// lint-long-file-override allow-max-lines=300
use std::io::Cursor;

use json_store::JsonStoreError;
use serde_json::json;
use tempfile::TempDir;

use crate::{
    error::Error,
    keystore::{
        KeyStore, StoredArgon2Algorithm, StoredKdf, StoredWallet, decrypt_private_key,
        encrypt_private_key, encrypt_private_key_with_kdf, find_wallet, load_keystore,
        next_wallet_name, prompt_wallet_name_with, validate_new_password, validate_wallet_name,
        wallet_address,
    },
};

const PRIVATE_KEY: &str = "4f3edf983ac636a65a842ce7c78d9aa706d3b113bce036f6c4d1f06b2d1f6f9d";

#[tokio::test]
async fn persists_wallet_store_with_kdf_metadata() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let store = load_keystore(temp_dir.path())
        .await
        .expect("load keystore store");
    let secret_key = hex::decode(PRIVATE_KEY).expect("decode secret key");
    let encrypted_private_key =
        encrypt_private_key(&secret_key, "beam-password").expect("encrypt secret key");

    store
        .set(KeyStore {
            wallets: vec![StoredWallet {
                address: format!(
                    "{:#x}",
                    wallet_address(&secret_key).expect("wallet address")
                ),
                encrypted_key: encrypted_private_key.encrypted_key,
                name: "alice".to_string(),
                salt: encrypted_private_key.salt,
                kdf: encrypted_private_key.kdf,
            }],
        })
        .await
        .expect("persist keystore");

    let persisted = std::fs::read_to_string(temp_dir.path().join("wallets.json"))
        .expect("read persisted keystore");
    let persisted = serde_json::from_str::<serde_json::Value>(&persisted)
        .expect("parse persisted keystore json");

    assert_eq!(
        persisted["wallets"][0]["kdf"],
        json!({
            "algorithm": "argon2id",
            "memory_kib": 19 * 1024,
            "parallelism": 1,
            "time_cost": 2,
            "type": "argon2",
            "version": 0x13,
        })
    );

    let reloaded = load_keystore(temp_dir.path())
        .await
        .expect("reload keystore")
        .get()
        .await;

    assert_eq!(reloaded.wallets.len(), 1);
    assert_eq!(reloaded.wallets[0].kdf, StoredKdf::current());
    assert_eq!(reloaded.wallets[0].name, "alice");
}

#[tokio::test]
async fn rejects_invalid_persisted_keystore_json() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_path = temp_dir.path().join("wallets.json");
    std::fs::write(&file_path, "{ invalid json").expect("write invalid keystore");

    let err = match load_keystore(temp_dir.path()).await {
        Ok(_) => panic!("expected invalid keystore to fail"),
        Err(err) => err,
    };

    match err {
        Error::Internal(internal) => match internal.recursive_downcast_ref::<JsonStoreError>() {
            Some(JsonStoreError::Deserialization { path, .. }) => assert_eq!(path, &file_path),
            other => panic!("unexpected json store error: {other:?}"),
        },
        other => panic!("unexpected error: {other:?}"),
    }

    let content = std::fs::read_to_string(&file_path).expect("read invalid keystore");
    assert_eq!(content, "{ invalid json");
}

#[test]
fn encrypts_and_decrypts_private_keys() {
    let secret_key = hex::decode(PRIVATE_KEY).expect("decode secret key");
    let encrypted_private_key =
        encrypt_private_key(&secret_key, "beam-password").expect("encrypt secret key");
    let wallet = StoredWallet {
        address: format!(
            "{:#x}",
            wallet_address(&secret_key).expect("wallet address")
        ),
        encrypted_key: encrypted_private_key.encrypted_key,
        name: "alice".to_string(),
        salt: encrypted_private_key.salt,
        kdf: encrypted_private_key.kdf,
    };

    assert_eq!(wallet.kdf, StoredKdf::current());

    let decrypted = decrypt_private_key(&wallet, "beam-password").expect("decrypt secret key");
    assert_eq!(decrypted, secret_key);

    let wrong_password =
        decrypt_private_key(&wallet, "wrong-password").expect_err("reject wrong password");
    assert!(matches!(wrong_password, Error::DecryptionFailed));
}

#[tokio::test]
async fn loads_legacy_wallets_without_kdf_metadata() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let secret_key = hex::decode(PRIVATE_KEY).expect("decode secret key");
    let address = format!(
        "{:#x}",
        wallet_address(&secret_key).expect("wallet address")
    );
    let encrypted_private_key =
        encrypt_private_key(&secret_key, "beam-password").expect("encrypt secret key");

    std::fs::write(
        temp_dir.path().join("wallets.json"),
        serde_json::to_string(&json!({
            "wallets": [{
                "address": address,
                "encrypted_key": encrypted_private_key.encrypted_key,
                "name": "alice",
                "salt": encrypted_private_key.salt,
            }],
        }))
        .expect("serialize legacy keystore"),
    )
    .expect("write legacy keystore");

    let wallet = load_keystore(temp_dir.path())
        .await
        .expect("load legacy keystore")
        .get()
        .await
        .wallets
        .into_iter()
        .next()
        .expect("load stored wallet");

    assert_eq!(wallet.kdf, StoredKdf::default());

    let decrypted = decrypt_private_key(&wallet, "beam-password").expect("decrypt legacy wallet");
    assert_eq!(decrypted, secret_key);
}

#[test]
fn decrypts_private_keys_with_persisted_argon2_parameters() {
    let secret_key = hex::decode(PRIVATE_KEY).expect("decode secret key");
    let kdf = StoredKdf::Argon2 {
        algorithm: StoredArgon2Algorithm::Argon2id,
        version: 0x13,
        memory_kib: 8 * 1024,
        parallelism: 1,
        time_cost: 2,
    };
    let encrypted_private_key = encrypt_private_key_with_kdf(&secret_key, "beam-password", kdf)
        .expect("encrypt secret key with persisted kdf");
    let wallet = StoredWallet {
        address: format!(
            "{:#x}",
            wallet_address(&secret_key).expect("wallet address")
        ),
        encrypted_key: encrypted_private_key.encrypted_key,
        name: "alice".to_string(),
        salt: encrypted_private_key.salt,
        kdf,
    };

    let decrypted = decrypt_private_key(&wallet, "beam-password")
        .expect("decrypt secret key with persisted kdf");

    assert_eq!(decrypted, secret_key);
}

#[test]
fn finds_wallets_by_address_selector() {
    let wallet = StoredWallet {
        address: "0x1111111111111111111111111111111111111111".to_string(),
        encrypted_key: "encrypted-key".to_string(),
        name: "alice".to_string(),
        salt: "salt".to_string(),
        kdf: StoredKdf::default(),
    };
    let wallets = [wallet];

    let resolved = find_wallet(&wallets, "0x1111111111111111111111111111111111111111")
        .expect("find wallet by address");

    assert_eq!(resolved.name, "alice");
}

#[test]
fn rejects_wallet_names_with_address_prefix() {
    let err = validate_wallet_name(&[], "0xalice", None)
        .expect_err("reject wallet names that look like addresses");

    assert!(matches!(
        err,
        Error::WalletNameStartsWithAddressPrefix { name } if name == "0xalice"
    ));
}

#[test]
fn finds_the_first_available_default_wallet_name() {
    let store = KeyStore {
        wallets: vec![
            StoredWallet {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                encrypted_key: "encrypted-key".to_string(),
                name: "wallet-1".to_string(),
                salt: "salt".to_string(),
                kdf: StoredKdf::default(),
            },
            StoredWallet {
                address: "0x2222222222222222222222222222222222222222".to_string(),
                encrypted_key: "encrypted-key".to_string(),
                name: "wallet-3".to_string(),
                salt: "salt".to_string(),
                kdf: StoredKdf::default(),
            },
        ],
    };

    assert_eq!(next_wallet_name(&store), "wallet-2");
}

#[test]
fn prompt_wallet_name_uses_default_when_input_is_empty() {
    let mut input = Cursor::new("\n");
    let mut output = Vec::new();

    let name = prompt_wallet_name_with("wallet-2", &mut input, &mut output)
        .expect("resolve default wallet name");

    assert_eq!(name, "wallet-2");
    assert_eq!(
        String::from_utf8(output).expect("decode prompt"),
        "beam wallet name [wallet-2]: "
    );
}

#[test]
fn prompt_wallet_name_errors_when_input_is_closed() {
    let mut input = Cursor::new("");
    let mut output = Vec::new();

    let err = prompt_wallet_name_with("wallet-2", &mut input, &mut output)
        .expect_err("closed stdin should not accept the default wallet name");

    assert!(matches!(
        err,
        Error::PromptClosed { label } if label == "beam wallet name"
    ));
    assert_eq!(
        String::from_utf8(output).expect("decode prompt"),
        "beam wallet name [wallet-2]: "
    );
}

#[test]
fn prompt_wallet_name_accepts_custom_input() {
    let mut input = Cursor::new("primary\n");
    let mut output = Vec::new();

    let name = prompt_wallet_name_with("wallet-2", &mut input, &mut output)
        .expect("resolve prompted wallet name");

    assert_eq!(name, "primary");
}

#[test]
fn rejects_blank_new_passwords() {
    for password in ["", " \t "] {
        let err = validate_new_password(password, password)
            .expect_err("reject empty or whitespace-only passwords");

        assert!(matches!(err, Error::PasswordBlank));
    }
}
