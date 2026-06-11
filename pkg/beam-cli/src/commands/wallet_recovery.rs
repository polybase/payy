use std::{fs, io::Read, path::Path};

use contextful::ResultContextExt;
use contracts::Address;
use serde_json::json;

#[cfg(test)]
use crate::commands::wallet::store_wallet_with_password;
use crate::{
    cli::RecoveryPhraseSourceArgs,
    commands::wallet::store_wallet,
    error::{Error, Result},
    human_output::sanitize_control_chars,
    keystore::{
        decrypt_private_key, prompt_existing_password, prompt_recovery_phrase, wallet_address,
    },
    output::CommandOutput,
    recovery_phrase::{private_key_to_recovery_phrase, recovery_phrase_to_private_key},
    runtime::BeamApp,
};

const RECOVERY_PHRASE_KIND: &str = "evm_private_key_bip39_entropy";
const RECOVERY_PHRASE_SEMANTIC_WARNING: &str = concat!(
    "This phrase is a direct BIP39 encoding of the raw EVM private key. ",
    "It is not a MetaMask or HD-wallet seed phrase; Beam does not use a derivation path, ",
    "account index, or seed expansion."
);

pub(crate) async fn export_recovery_phrase(
    app: &BeamApp,
    requested_wallet: Option<String>,
) -> Result<()> {
    let password = prompt_existing_password()?;
    export_recovery_phrase_output(app, requested_wallet, &password)
        .await?
        .print(app.output_mode)
}

pub(crate) async fn export_recovery_phrase_output(
    app: &BeamApp,
    requested_wallet: Option<String>,
    password: &str,
) -> Result<CommandOutput> {
    let wallet = match requested_wallet {
        Some(selector) => app.resolve_wallet(&selector).await?,
        None => app.active_wallet().await?,
    };
    let private_key = decrypt_private_key(&wallet, password)?;
    let recovery_phrase = private_key_to_recovery_phrase(&private_key)?;
    let display_name = sanitize_control_chars(&wallet.name);
    let default_output = format!(
        "Recovery phrase for {display_name} ({}):\n{}\n\n{}\n\nStore this phrase securely. Anyone with it can control this wallet.",
        wallet.address, recovery_phrase, RECOVERY_PHRASE_SEMANTIC_WARNING
    );
    let markdown_output = format!(
        "Recovery phrase for `{display_name}` (`{}`):\n\n```text\n{}\n```\n\n{}\n\nStore this phrase securely. Anyone with it can control this wallet.",
        wallet.address, recovery_phrase, RECOVERY_PHRASE_SEMANTIC_WARNING
    );

    Ok(CommandOutput::new(
        default_output,
        json!({
            "address": wallet.address,
            "hd_seed_phrase": false,
            "name": wallet.name,
            "recovery_phrase": recovery_phrase.clone(),
            "recovery_phrase_kind": RECOVERY_PHRASE_KIND,
            "warning": RECOVERY_PHRASE_SEMANTIC_WARNING,
        }),
    )
    .compact(recovery_phrase)
    .markdown(markdown_output))
}

pub(crate) async fn import_recovery_phrase(
    app: &BeamApp,
    requested_name: Option<String>,
    phrase_source: &RecoveryPhraseSourceArgs,
    expected_address: Option<&str>,
) -> Result<()> {
    let recovery_phrase = read_recovery_phrase(phrase_source)?;
    let secret_key = recovery_phrase_to_private_key(&recovery_phrase)?;
    let derived_address = recovery_phrase_wallet_address(&secret_key)?;
    validate_expected_recovery_phrase_address(expected_address, &derived_address)?;
    eprintln!("{}", recovery_phrase_import_warning(&derived_address));
    store_wallet(app, requested_name, &secret_key).await
}

#[cfg(test)]
pub(crate) async fn import_recovery_phrase_with_password(
    app: &BeamApp,
    requested_name: Option<String>,
    recovery_phrase: &str,
    password: &str,
    expected_address: Option<&str>,
) -> Result<CommandOutput> {
    let secret_key = recovery_phrase_to_private_key(recovery_phrase)?;
    let derived_address = recovery_phrase_wallet_address(&secret_key)?;
    validate_expected_recovery_phrase_address(expected_address, &derived_address)?;
    store_wallet_with_password(app, requested_name, &secret_key, password).await
}

pub(crate) fn recovery_phrase_import_warning(derived_address: &str) -> String {
    format!(
        "Recovery phrase will restore EVM wallet: {derived_address}\n{RECOVERY_PHRASE_SEMANTIC_WARNING}\nContinue only if this address is expected. Press Ctrl-C to cancel."
    )
}

fn validate_expected_recovery_phrase_address(
    expected_address: Option<&str>,
    derived_address: &str,
) -> Result<()> {
    let Some(expected_address) = expected_address else {
        return Ok(());
    };

    let expected = expected_address
        .parse::<Address>()
        .map_err(|_| Error::InvalidAddress {
            value: expected_address.to_string(),
        })?;
    let expected = format!("{expected:#x}");
    if expected != derived_address {
        return Err(Error::RecoveryPhraseAddressMismatch {
            derived: derived_address.to_string(),
            expected,
        });
    }

    Ok(())
}

fn recovery_phrase_wallet_address(secret_key: &[u8]) -> Result<String> {
    Ok(format!("{:#x}", wallet_address(secret_key)?))
}

fn read_recovery_phrase(phrase_source: &RecoveryPhraseSourceArgs) -> Result<String> {
    if phrase_source.phrase_stdin {
        return read_recovery_phrase_from_stdin();
    }

    if let Some(fd) = phrase_source.phrase_fd {
        return read_recovery_phrase_from_fd(fd);
    }

    prompt_recovery_phrase()
}

fn read_recovery_phrase_from_stdin() -> Result<String> {
    let mut recovery_phrase = String::new();
    std::io::stdin()
        .read_to_string(&mut recovery_phrase)
        .context("read beam recovery phrase from stdin")?;
    Ok(recovery_phrase)
}

pub(crate) fn read_recovery_phrase_from_fd(fd: u32) -> Result<String> {
    let path = Path::new("/dev/fd").join(fd.to_string());
    Ok(fs::read_to_string(path).context("read beam recovery phrase from file descriptor")?)
}
