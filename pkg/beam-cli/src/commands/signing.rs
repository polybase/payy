use crate::{
    error::Result,
    keystore::{StoredWallet, decrypt_private_key, prompt_existing_password},
    runtime::BeamApp,
    signer::KeySigner,
};

pub(crate) async fn prompt_active_signer(app: &BeamApp) -> Result<KeySigner> {
    prompt_active_signer_with(app, prompt_existing_password).await
}

pub(crate) async fn prompt_active_signer_with<F>(
    app: &BeamApp,
    prompt_password: F,
) -> Result<KeySigner>
where
    F: FnOnce() -> Result<String>,
{
    let wallet = app.active_wallet().await?;
    signer_for_wallet_with(wallet, prompt_password)
}

fn signer_for_wallet_with<F>(wallet: StoredWallet, prompt_password: F) -> Result<KeySigner>
where
    F: FnOnce() -> Result<String>,
{
    let password = prompt_password()?;
    let secret_key = decrypt_private_key(&wallet, &password)?;
    KeySigner::from_slice(&secret_key)
}
