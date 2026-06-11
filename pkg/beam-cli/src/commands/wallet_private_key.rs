use contracts::Secp256k1SecretKey;
use serde_json::json;

use crate::{
    error::{Error, Result},
    human_output::sanitize_control_chars,
    keystore::{StoredWallet, decrypt_private_key, prompt_existing_password},
    output::CommandOutput,
    runtime::BeamApp,
};

const PRIVATE_KEY_WARNING: &str =
    "Store this key securely. Anyone with it can control this wallet.";

pub(crate) async fn export_private_key(app: &BeamApp, wallet: Option<String>) -> Result<()> {
    let wallet = resolve_export_wallet(app, wallet.as_deref()).await?;
    let password = prompt_existing_password()?;

    export_private_key_output(&wallet, &password)?.print(app.output_mode)
}

#[cfg(test)]
pub(crate) async fn export_private_key_output_for_selector(
    app: &BeamApp,
    wallet: Option<&str>,
    password: &str,
) -> Result<CommandOutput> {
    let wallet = resolve_export_wallet(app, wallet).await?;
    export_private_key_output(&wallet, password)
}

async fn resolve_export_wallet(app: &BeamApp, wallet: Option<&str>) -> Result<StoredWallet> {
    match wallet {
        Some(selector) => app.resolve_wallet(selector).await,
        None => app.active_wallet().await,
    }
}

pub(crate) fn export_private_key_output(
    wallet: &StoredWallet,
    password: &str,
) -> Result<CommandOutput> {
    let secret_key = decrypt_private_key(wallet, password)?;
    render_export_private_key_output(wallet, &secret_key)
}

fn render_export_private_key_output(
    wallet: &StoredWallet,
    secret_key: &[u8],
) -> Result<CommandOutput> {
    let _ = Secp256k1SecretKey::from_slice(secret_key).map_err(|_| Error::InvalidPrivateKey)?;
    let private_key = format!("0x{}", hex::encode(secret_key));
    let name = sanitize_control_chars(&wallet.name);
    let default = format!(
        "Private key for {name} ({}):\n{private_key}\n\n{PRIVATE_KEY_WARNING}",
        wallet.address
    );
    let markdown = format!(
        "Private key for {name} (`{}`):\n\n```text\n{private_key}\n```\n\n{PRIVATE_KEY_WARNING}",
        wallet.address
    );

    Ok(CommandOutput::new(
        default,
        json!({
            "address": &wallet.address,
            "name": &wallet.name,
            "private_key": private_key,
            "warning": PRIVATE_KEY_WARNING,
        }),
    )
    .compact(private_key)
    .markdown(markdown))
}
