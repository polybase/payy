// lint-long-file-override allow-max-lines=400
use contextful::ResultContextExt;
use serde_json::json;

use super::{
    wallet_private_key, wallet_recovery,
    wallet_secret::{generate_secret_key, load_secret_key},
};
#[cfg(test)]
use crate::keystore::validate_new_password;
use crate::{
    cli::{PrivateKeySourceArgs, WalletAction},
    ens::{import_wallet_name, validate_wallet_name_for_address},
    error::{Error, Result},
    human_output::{normalize_human_name, sanitize_control_chars},
    keystore::{
        StoredWallet, encrypt_private_key, next_wallet_name, prompt_new_password,
        prompt_wallet_name, wallet_address,
    },
    output::CommandOutput,
    runtime::BeamApp,
};

pub async fn run(app: &BeamApp, action: WalletAction) -> Result<()> {
    match action {
        WalletAction::Create { name } => create_wallet(app, name).await,
        WalletAction::Import {
            private_key_source,
            name,
        } => import_wallet(app, name, &private_key_source).await,
        WalletAction::ExportPrivateKey { wallet } => {
            wallet_private_key::export_private_key(app, wallet).await
        }
        WalletAction::ExportRecoveryPhrase { wallet } => {
            wallet_recovery::export_recovery_phrase(app, wallet).await
        }
        WalletAction::ImportRecoveryPhrase {
            expected_address,
            phrase_source,
            name,
        } => {
            wallet_recovery::import_recovery_phrase(
                app,
                name,
                &phrase_source,
                expected_address.as_deref(),
            )
            .await
        }
        WalletAction::List => list_wallets(app).await,
        WalletAction::Rename { name, new_name } => rename_wallet(app, &name, &new_name).await,
        WalletAction::Address { private_key_source } => {
            show_address(app, &private_key_source).await
        }
        WalletAction::Use { name } => use_wallet(app, &name).await,
    }
}

async fn create_wallet(app: &BeamApp, requested_name: Option<String>) -> Result<()> {
    let requested_name = match requested_name {
        Some(name) => Some(name),
        None => {
            let keystore = app.keystore_store.get().await;
            Some(prompt_wallet_name(&next_wallet_name(&keystore))?)
        }
    };
    let secret_key = generate_secret_key();
    store_wallet(app, requested_name, &secret_key).await
}

async fn import_wallet(
    app: &BeamApp,
    requested_name: Option<String>,
    private_key_source: &PrivateKeySourceArgs,
) -> Result<()> {
    let secret_key = load_secret_key(private_key_source)?;
    store_wallet(app, requested_name, &secret_key).await
}

async fn list_wallets(app: &BeamApp) -> Result<()> {
    let config = app.config_store.get().await;
    let keystore = app.keystore_store.get().await;

    if keystore.wallets.is_empty() {
        return CommandOutput::message("No wallets configured.").print(app.output_mode);
    }

    let default_name = config.default_wallet.as_deref();
    let lines = keystore
        .wallets
        .iter()
        .map(|wallet| {
            let suffix = if default_name.is_some_and(|name| wallet.name.eq_ignore_ascii_case(name))
            {
                " (default)"
            } else {
                ""
            };
            let name = sanitize_control_chars(&wallet.name);
            format!("{name}  {}{}", wallet.address, suffix)
        })
        .collect::<Vec<_>>();
    let value = json!({
        "wallets": keystore.wallets.iter().map(|wallet| {
            json!({
                "address": wallet.address,
                "is_default": default_name.is_some_and(|name| wallet.name.eq_ignore_ascii_case(name)),
                "name": wallet.name,
            })
        }).collect::<Vec<_>>()
    });

    CommandOutput::new(lines.join("\n"), value)
        .compact(lines.join(" | "))
        .markdown(
            keystore
                .wallets
                .iter()
                .map(|wallet| {
                    let name = sanitize_control_chars(&wallet.name);
                    format!("- `{name}` `{}`", wallet.address)
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .print(app.output_mode)
}

async fn show_address(app: &BeamApp, private_key_source: &PrivateKeySourceArgs) -> Result<()> {
    let secret_key = load_secret_key(private_key_source)?;
    let address = format!("{:#x}", wallet_address(&secret_key)?);
    CommandOutput::new(address.clone(), json!({ "address": address }))
        .compact(address)
        .print(app.output_mode)
}

pub(crate) async fn rename_wallet(app: &BeamApp, name: &str, new_name: &str) -> Result<()> {
    let wallet = app.resolve_wallet(name).await?;
    let address = wallet.address.clone();
    let old_name = wallet.name.clone();
    let keystore = app.keystore_store.get().await;
    let new_name = normalize_wallet_name(new_name)?;

    validate_wallet_name_for_address(app, &keystore.wallets, &new_name, Some(&address), &address)
        .await?;

    let address_for_store = address.clone();
    let new_name_for_store = new_name.clone();
    app.keystore_store
        .update(move |store| {
            if let Some(wallet) = store
                .wallets
                .iter_mut()
                .find(|wallet| wallet.address == address_for_store)
            {
                wallet.name = new_name_for_store.clone();
            }
        })
        .await
        .context("persist beam wallet rename")?;

    let old_name_for_config = old_name.clone();
    let new_name_for_config = new_name.clone();
    app.config_store
        .update(move |config| {
            if config
                .default_wallet
                .as_ref()
                .is_some_and(|default_wallet| {
                    default_wallet.eq_ignore_ascii_case(&old_name_for_config)
                })
            {
                config.default_wallet = Some(new_name_for_config.clone());
            }
        })
        .await
        .context("persist beam default wallet")?;

    let display_old_name = sanitize_control_chars(&old_name);
    CommandOutput::new(
        format!("Renamed wallet {display_old_name} to {new_name} ({address})"),
        json!({
            "address": address,
            "name": new_name,
            "previous_name": old_name,
        }),
    )
    .compact(format!("{new_name} {address}"))
    .print(app.output_mode)
}

async fn use_wallet(app: &BeamApp, name: &str) -> Result<()> {
    let wallet = app.resolve_wallet(name).await?;
    let name = wallet.name.clone();

    app.config_store
        .update(|config| config.default_wallet = Some(name.clone()))
        .await
        .context("persist beam default wallet")?;

    let name = sanitize_control_chars(&name);
    CommandOutput::new(
        format!("Default wallet set to {name} ({})", wallet.address),
        json!({
            "address": wallet.address,
            "name": wallet.name,
        }),
    )
    .compact(name)
    .print(app.output_mode)
}

pub(super) async fn store_wallet(
    app: &BeamApp,
    requested_name: Option<String>,
    secret_key: &[u8],
) -> Result<()> {
    let prepared = prepare_wallet_store(app, requested_name, secret_key).await?;
    let password = prompt_new_password()?;
    persist_prepared_wallet(app, prepared, secret_key, &password)
        .await?
        .print(app.output_mode)
}

#[cfg(test)]
pub(super) async fn store_wallet_with_password(
    app: &BeamApp,
    requested_name: Option<String>,
    secret_key: &[u8],
    password: &str,
) -> Result<CommandOutput> {
    let prepared = prepare_wallet_store(app, requested_name, secret_key).await?;
    validate_new_password(password, password)?;

    persist_prepared_wallet(app, prepared, secret_key, password).await
}

struct PreparedWalletStore {
    address: String,
    name: String,
}

async fn prepare_wallet_store(
    app: &BeamApp,
    requested_name: Option<String>,
    secret_key: &[u8],
) -> Result<PreparedWalletStore> {
    let keystore = app.keystore_store.get().await;
    let address = wallet_address(secret_key)?;
    let name = import_wallet_name(app, &keystore, requested_name, address).await?;
    let name = normalize_wallet_name(&name)?;
    let address = format!("{address:#x}");
    validate_wallet_name_for_address(app, &keystore.wallets, &name, None, &address).await?;
    if keystore
        .wallets
        .iter()
        .any(|wallet| wallet.address == address)
    {
        return Err(Error::WalletAddressAlreadyExists { address });
    }

    Ok(PreparedWalletStore { address, name })
}

async fn persist_prepared_wallet(
    app: &BeamApp,
    prepared: PreparedWalletStore,
    secret_key: &[u8],
    password: &str,
) -> Result<CommandOutput> {
    let PreparedWalletStore { address, name } = prepared;
    let encrypted_private_key = encrypt_private_key(secret_key, password)?;
    let wallet = StoredWallet {
        address: address.clone(),
        encrypted_key: encrypted_private_key.encrypted_key,
        name: name.clone(),
        salt: encrypted_private_key.salt,
        kdf: encrypted_private_key.kdf,
    };
    let wallet_to_store = wallet.clone();

    app.keystore_store
        .update(move |store| store.wallets.push(wallet_to_store))
        .await
        .context("persist beam wallet")?;

    if app.config_store.get().await.default_wallet.is_none() {
        app.config_store
            .update(|config| config.default_wallet = Some(name.clone()))
            .await
            .context("persist beam default wallet")?;
    }

    let display_name = sanitize_control_chars(&wallet.name);
    Ok(CommandOutput::new(
        format!("Created wallet {display_name} ({address})"),
        json!({
            "address": wallet.address,
            "name": wallet.name,
        }),
    )
    .compact(format!("{display_name} {address}")))
}

pub(crate) fn normalize_wallet_name(name: &str) -> Result<String> {
    normalize_human_name(name).ok_or(Error::WalletNameBlank)
}
