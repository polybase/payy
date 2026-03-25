use std::sync::atomic::{AtomicUsize, Ordering};

use super::{
    ens::{set_ethereum_rpc, spawn_ens_rpc_server},
    fixtures::test_app,
};
use crate::{
    commands::signing::prompt_active_signer_with,
    error::Error,
    keystore::{KeyStore, StoredWallet, decrypt_private_key, encrypt_private_key, wallet_address},
    runtime::{BeamApp, InvocationOverrides},
    signer::Signer,
};

const PRIVATE_KEY: &str = "4f3edf983ac636a65a842ce7c78d9aa706d3b113bce036f6c4d1f06b2d1f6f9d";
const TAMPERED_ADDRESS: &str = "0x1111111111111111111111111111111111111111";

async fn seed_wallet(app: &BeamApp, wallet: StoredWallet) {
    let default_wallet = wallet.name.clone();

    app.keystore_store
        .set(KeyStore {
            wallets: vec![wallet],
        })
        .await
        .expect("persist keystore");

    app.config_store
        .update(move |config| config.default_wallet = Some(default_wallet.clone()))
        .await
        .expect("persist default wallet");
}

fn secret_key() -> Vec<u8> {
    hex::decode(PRIVATE_KEY).expect("decode secret key")
}

fn derived_address() -> String {
    format!(
        "{:#x}",
        wallet_address(&secret_key()).expect("derive wallet address")
    )
}

fn encrypted_wallet(name: &str, address: &str) -> StoredWallet {
    let encrypted_private_key =
        encrypt_private_key(&secret_key(), "beam-password").expect("encrypt secret key");

    StoredWallet {
        address: address.to_string(),
        encrypted_key: encrypted_private_key.encrypted_key,
        name: name.to_string(),
        salt: encrypted_private_key.salt,
        kdf: encrypted_private_key.kdf,
    }
}

#[test]
fn decrypt_private_key_rejects_stored_address_mismatch() {
    let expected_address = derived_address();
    let wallet = encrypted_wallet("alice", TAMPERED_ADDRESS);

    let err = decrypt_private_key(&wallet, "beam-password")
        .expect_err("reject tampered keystore address");

    assert!(matches!(
        err,
        Error::StoredWalletAddressMismatch { derived, stored }
            if stored == TAMPERED_ADDRESS && derived == expected_address
    ));
}

#[tokio::test]
async fn active_signer_uses_verified_wallet_key() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let expected_address = derived_address();
    seed_wallet(&app, encrypted_wallet("alice", &expected_address)).await;

    let signer = app
        .active_signer("beam-password")
        .await
        .expect("load verified active signer");

    assert_eq!(format!("{:#x}", signer.address()), expected_address);
}

#[tokio::test]
async fn active_signer_rejects_tampered_keystore_address() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let expected_address = derived_address();
    seed_wallet(&app, encrypted_wallet("alice", TAMPERED_ADDRESS)).await;

    let err = match app.active_signer("beam-password").await {
        Ok(_) => panic!("expected tampered active signer to fail"),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        Error::StoredWalletAddressMismatch { derived, stored }
            if stored == TAMPERED_ADDRESS && derived == expected_address
    ));
}

#[tokio::test]
async fn prompted_signer_skips_password_when_raw_sender_has_no_local_wallet() {
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some(TAMPERED_ADDRESS.to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    let prompts = AtomicUsize::new(0);

    let err = match prompt_active_signer_with(&app, || {
        prompts.fetch_add(1, Ordering::SeqCst);
        Ok("beam-password".to_string())
    })
    .await
    {
        Ok(_) => panic!("expected raw sender without local wallet to fail"),
        Err(err) => err,
    };

    assert!(matches!(err, Error::WalletNotFound { selector } if selector == TAMPERED_ADDRESS));
    assert_eq!(prompts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn prompted_signer_skips_password_when_ens_sender_has_no_local_wallet() {
    let (rpc_url, _calls, server) = spawn_ens_rpc_server("alice.eth", TAMPERED_ADDRESS).await;
    let (_temp_dir, app) = test_app(InvocationOverrides {
        from: Some("alice.eth".to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    set_ethereum_rpc(&app, &rpc_url).await;
    let prompts = AtomicUsize::new(0);

    let err = match prompt_active_signer_with(&app, || {
        prompts.fetch_add(1, Ordering::SeqCst);
        Ok("beam-password".to_string())
    })
    .await
    {
        Ok(_) => panic!("expected ens sender without local wallet to fail"),
        Err(err) => err,
    };
    server.abort();

    assert!(matches!(err, Error::WalletNotFound { selector } if selector == "alice.eth"));
    assert_eq!(prompts.load(Ordering::SeqCst), 0);
}
