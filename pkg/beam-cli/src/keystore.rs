// lint-long-file-override allow-max-lines=300
mod crypto;

use std::{
    io::{self, BufRead, Write},
    path::Path,
};

use argon2::{Algorithm, Argon2, Params, Version};
use contextful::{ErrorContextExt, ResultContextExt};
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    prompts::prompt_with_default_with,
};

const ENCRYPTION_KEY_LEN: usize = 32;

#[cfg(test)]
pub(crate) use crypto::encrypt_private_key_with_kdf;
pub use crypto::{decrypt_private_key, encrypt_private_key, wallet_address};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyStore {
    pub wallets: Vec<StoredWallet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredWallet {
    pub address: String,
    pub encrypted_key: String,
    pub name: String,
    pub salt: String,
    #[serde(default)]
    pub kdf: StoredKdf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum StoredKdf {
    #[serde(rename = "argon2")]
    Argon2 {
        algorithm: StoredArgon2Algorithm,
        version: u32,
        memory_kib: u32,
        parallelism: u32,
        time_cost: u32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StoredArgon2Algorithm {
    #[serde(rename = "argon2d")]
    Argon2d,
    #[serde(rename = "argon2i")]
    Argon2i,
    #[serde(rename = "argon2id")]
    Argon2id,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedPrivateKey {
    pub encrypted_key: String,
    pub kdf: StoredKdf,
    pub salt: String,
}

impl Default for StoredKdf {
    fn default() -> Self {
        // Missing `kdf` metadata means the wallet predates versioned KDF storage.
        Self::legacy_argon2id()
    }
}

impl From<StoredArgon2Algorithm> for Algorithm {
    fn from(algorithm: StoredArgon2Algorithm) -> Self {
        match algorithm {
            StoredArgon2Algorithm::Argon2d => Algorithm::Argon2d,
            StoredArgon2Algorithm::Argon2i => Algorithm::Argon2i,
            StoredArgon2Algorithm::Argon2id => Algorithm::Argon2id,
        }
    }
}

impl StoredKdf {
    pub(crate) fn current() -> Self {
        Self::legacy_argon2id()
    }

    fn legacy_argon2id() -> Self {
        Self::Argon2 {
            algorithm: StoredArgon2Algorithm::Argon2id,
            version: 0x13,
            memory_kib: 19 * 1024,
            parallelism: 1,
            time_cost: 2,
        }
    }

    fn derive_key(self, password: &str, salt: &[u8]) -> Result<[u8; ENCRYPTION_KEY_LEN]> {
        match self {
            Self::Argon2 {
                algorithm,
                version,
                memory_kib,
                parallelism,
                time_cost,
            } => derive_argon2_key(
                password,
                salt,
                algorithm.into(),
                version,
                memory_kib,
                parallelism,
                time_cost,
            ),
        }
    }
}

pub async fn load_keystore(root: &Path) -> Result<JsonStore<KeyStore>> {
    Ok(JsonStore::new_with_invalid_json_behavior_and_access(
        root,
        "wallets.json",
        InvalidJsonBehavior::Error,
        FileAccess::OwnerOnly,
    )
    .await
    .context("load beam keystore")?)
}

pub fn prompt_existing_password() -> Result<String> {
    prompt_secret("beam password: ", "read beam password")
}

pub fn prompt_private_key() -> Result<String> {
    prompt_secret("beam private key: ", "read beam private key")
}

pub fn prompt_wallet_name(default_name: &str) -> Result<String> {
    let (stdin, stderr) = (std::io::stdin(), std::io::stderr());
    prompt_wallet_name_with(default_name, &mut stdin.lock(), &mut stderr.lock())
}

pub fn prompt_new_password() -> Result<String> {
    let password = prompt_secret("beam password: ", "read beam password")?;
    let confirmation = prompt_secret("confirm beam password: ", "read beam password confirmation")?;
    validate_new_password(&password, &confirmation).map(|_| password)
}

fn prompt_secret(prompt: &str, context: &'static str) -> Result<String> {
    prompt_secret_with(|| prompt_password(prompt), context)
}

pub(crate) fn prompt_secret_with<F>(prompt: F, context: &'static str) -> Result<String>
where
    F: FnOnce() -> io::Result<String>,
{
    match prompt() {
        Ok(value) => Ok(value),
        Err(err) if err.kind() == io::ErrorKind::Interrupted => Err(Error::Interrupted),
        Err(err) => Err(err.context(context).into()),
    }
}

pub(crate) fn validate_new_password(password: &str, confirmation: &str) -> Result<()> {
    match (password.trim().is_empty(), password == confirmation) {
        (true, _) => Err(Error::PasswordBlank),
        (false, true) => Ok(()),
        (false, false) => Err(Error::PasswordConfirmationMismatch),
    }
}

pub(crate) fn prompt_wallet_name_with<R, W>(
    default_name: &str,
    input: &mut R,
    output: &mut W,
) -> Result<String>
where
    R: BufRead,
    W: Write,
{
    prompt_with_default_with("beam wallet name", default_name, input, output)
}

pub fn next_wallet_name(store: &KeyStore) -> String {
    let mut index = 1;
    loop {
        let name = format!("wallet-{index}");
        if store
            .wallets
            .iter()
            .all(|wallet| !wallet.name.eq_ignore_ascii_case(&name))
        {
            return name;
        }
        index += 1;
    }
}

pub fn is_address_selector(value: &str) -> bool {
    value
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("0x"))
}

pub fn validate_wallet_name(
    wallets: &[StoredWallet],
    name: &str,
    current_address: Option<&str>,
) -> Result<()> {
    let name = match name.trim() {
        "" => return Err(Error::WalletNameBlank),
        name => name,
    };

    if is_address_selector(name) {
        return Err(Error::WalletNameStartsWithAddressPrefix {
            name: name.to_string(),
        });
    }

    if wallets.iter().any(|wallet| {
        wallet.name.eq_ignore_ascii_case(name)
            && current_address.is_none_or(|address| wallet.address != address)
    }) {
        return Err(Error::WalletNameAlreadyExists {
            name: name.to_string(),
        });
    }

    Ok(())
}

pub fn find_wallet<'a>(wallets: &'a [StoredWallet], selector: &str) -> Result<&'a StoredWallet> {
    wallets
        .iter()
        .find(|wallet| {
            if is_address_selector(selector) {
                wallet.address.eq_ignore_ascii_case(selector)
            } else {
                wallet.name.eq_ignore_ascii_case(selector)
            }
        })
        .ok_or_else(|| Error::WalletNotFound {
            selector: selector.to_string(),
        })
}

fn derive_argon2_key(
    password: &str,
    salt: &[u8],
    algorithm: Algorithm,
    version: u32,
    memory_kib: u32,
    parallelism: u32,
    time_cost: u32,
) -> Result<[u8; ENCRYPTION_KEY_LEN]> {
    let params = Params::new(memory_kib, time_cost, parallelism, Some(ENCRYPTION_KEY_LEN))
        .map_err(|_| Error::KeyDerivationFailed)?;
    let version = Version::try_from(version).map_err(|_| Error::KeyDerivationFailed)?;
    let mut key = [0u8; ENCRYPTION_KEY_LEN];
    Argon2::new(algorithm, version, params)
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| Error::KeyDerivationFailed)?;
    Ok(key)
}
