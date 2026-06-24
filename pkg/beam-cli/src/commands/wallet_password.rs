use contextful::ResultContextExt;
use serde_json::json;

use crate::{
    error::{Error, Result},
    human_output::sanitize_control_chars,
    keystore::{
        decrypt_private_key, encrypt_private_key, prompt_existing_password, prompt_new_password,
        validate_new_password,
    },
    output::CommandOutput,
    runtime::BeamApp,
};

pub(crate) async fn change_wallet_password(
    app: &BeamApp,
    wallet_selector: Option<&str>,
) -> Result<()> {
    let current_password = prompt_existing_password()?;
    let new_password = prompt_new_password()?;
    change_wallet_password_with_passwords(app, wallet_selector, &current_password, &new_password)
        .await?
        .print(app.output_mode)
}

pub(crate) async fn change_wallet_password_with_passwords(
    app: &BeamApp,
    wallet_selector: Option<&str>,
    current_password: &str,
    new_password: &str,
) -> Result<CommandOutput> {
    validate_new_password(new_password, new_password)?;
    let wallet = match wallet_selector {
        Some(selector) => app.resolve_wallet(selector).await?,
        None => app.active_wallet().await?,
    };
    let secret_key = decrypt_private_key(&wallet, current_password)?;
    let encrypted_private_key = encrypt_private_key(&secret_key, new_password)?;

    let address = wallet.address.clone();
    let mut keystore = app.keystore_store.get().await;
    let stored_wallet = keystore
        .wallets
        .iter_mut()
        .find(|stored_wallet| stored_wallet.address.eq_ignore_ascii_case(&address))
        .ok_or_else(|| Error::WalletNotFound {
            selector: address.clone(),
        })?;
    stored_wallet.encrypted_key = encrypted_private_key.encrypted_key;
    stored_wallet.salt = encrypted_private_key.salt;
    stored_wallet.kdf = encrypted_private_key.kdf;

    app.keystore_store
        .set(keystore)
        .await
        .context("persist beam wallet password change")?;

    let display_name = sanitize_control_chars(&wallet.name);
    Ok(CommandOutput::new(
        format!("Changed password for wallet {display_name} ({address})"),
        json!({
            "address": address,
            "name": wallet.name,
        }),
    )
    .compact(format!("{display_name} {address}")))
}
