use crate::{
    error::{Error, Result},
    keystore::{StoredWallet, decrypt_private_key, prompt_existing_password},
    profiles::{model::PublicSigningIntent, session, signer::ProfileSigner},
    runtime::BeamApp,
    signer::{KeySigner, Signer},
};

pub(crate) async fn prompt_active_signer(app: &BeamApp) -> Result<KeySigner> {
    prompt_active_signer_with(app, prompt_existing_password).await
}

pub(crate) async fn active_signing_address(app: &BeamApp) -> Result<contracts::Address> {
    match app.overrides.profile.as_deref() {
        Some(profile) => {
            let session = session::active(&app.paths.root, profile).await?;
            session
                .wallet_address
                .parse()
                .map_err(|_| Error::InvalidAddress {
                    value: session.wallet_address,
                })
        }
        None => app.active_address().await,
    }
}

pub(crate) async fn active_signer_for_intent(
    app: &BeamApp,
    intent: PublicSigningIntent,
) -> Result<Box<dyn Signer>> {
    match app.overrides.profile.as_deref() {
        Some(profile) => {
            let mut session = session::active(&app.paths.root, profile).await?;
            if let Ok(token) = std::env::var("BEAM_PROFILE_TOKEN") {
                session.token = token;
            }
            Ok(Box::new(ProfileSigner::new(session, intent)?))
        }
        None => Ok(Box::new(prompt_active_signer(app).await?)),
    }
}

pub(crate) async fn prompt_active_private_key(app: &BeamApp) -> Result<[u8; 32]> {
    let wallet = app.active_wallet().await?;
    let password = prompt_existing_password()?;
    let secret_key = decrypt_private_key(&wallet, &password)?;
    secret_key.try_into().map_err(|_| Error::InvalidPrivateKey)
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
