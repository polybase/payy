use super::fixtures::test_app_with_output;
use crate::{
    commands::wallet_password::change_wallet_password_with_passwords,
    error::Error,
    keystore::{KeyStore, StoredWallet, decrypt_private_key, encrypt_private_key, wallet_address},
    output::OutputMode,
    runtime::{BeamApp, InvocationOverrides},
};

const SECRET_KEY: [u8; 32] = [1u8; 32];

async fn seed_encrypted_wallet(app: &BeamApp, name: &str, password: &str) -> String {
    let encrypted_private_key =
        encrypt_private_key(&SECRET_KEY, password).expect("encrypt private key");
    let address = format!(
        "{:#x}",
        wallet_address(&SECRET_KEY).expect("derive wallet address")
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

    app.config_store
        .update(|config| config.default_wallet = Some(name.to_string()))
        .await
        .expect("persist default wallet");

    address
}

#[tokio::test]
async fn change_password_reencrypts_empty_password_wallet() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let address = seed_encrypted_wallet(&app, "alice", "").await;

    let output = change_wallet_password_with_passwords(&app, None, "", "beam-password")
        .await
        .expect("change wallet password");

    assert_eq!(output.value["address"], address);
    let keystore = app.keystore_store.get().await;
    let wallet = &keystore.wallets[0];
    assert!(matches!(
        decrypt_private_key(wallet, ""),
        Err(Error::DecryptionFailed)
    ));
    assert_eq!(
        decrypt_private_key(wallet, "beam-password").expect("decrypt with new password"),
        SECRET_KEY.to_vec()
    );
}

#[tokio::test]
async fn change_password_rejects_wrong_current_password_without_mutating_wallet() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    seed_encrypted_wallet(&app, "alice", "old-password").await;

    let err = change_wallet_password_with_passwords(
        &app,
        Some("alice"),
        "wrong-password",
        "new-password",
    )
    .await
    .expect_err("reject wrong current password");
    assert!(matches!(err, Error::DecryptionFailed));

    let keystore = app.keystore_store.get().await;
    let wallet = &keystore.wallets[0];
    assert_eq!(
        decrypt_private_key(wallet, "old-password").expect("decrypt with old password"),
        SECRET_KEY.to_vec()
    );
    assert!(matches!(
        decrypt_private_key(wallet, "new-password"),
        Err(Error::DecryptionFailed)
    ));
}
