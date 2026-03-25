use contextful::ResultContextExt;
use contracts::Address;
use encrypt::{EncryptedSymmetricData, Key, symmetric_decrypt, symmetric_encrypt};
use eth_util::secret_key_to_address;
use rand::RngCore;
use secp256k1::SecretKey;

use crate::error::{Error, Result};

use super::{EncryptedPrivateKey, StoredKdf, StoredWallet};

pub fn wallet_address(secret_key: &[u8]) -> Result<Address> {
    let secret_key = SecretKey::from_slice(secret_key).map_err(|_| Error::InvalidPrivateKey)?;
    Ok(secret_key_to_address(&secret_key))
}

pub fn encrypt_private_key(secret_key: &[u8], password: &str) -> Result<EncryptedPrivateKey> {
    encrypt_private_key_with_kdf(secret_key, password, StoredKdf::current())
}

pub(crate) fn encrypt_private_key_with_kdf(
    secret_key: &[u8],
    password: &str,
    kdf: StoredKdf,
) -> Result<EncryptedPrivateKey> {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let key = kdf.derive_key(password, &salt)?;
    let encrypted =
        symmetric_encrypt(Key::from_slice(&key), secret_key).context("encrypt beam private key")?;

    Ok(EncryptedPrivateKey {
        encrypted_key: hex::encode(encrypted.to_bytes()),
        kdf,
        salt: hex::encode(salt),
    })
}

pub fn decrypt_private_key(wallet: &StoredWallet, password: &str) -> Result<Vec<u8>> {
    let salt = hex::decode(&wallet.salt).context("decode beam wallet salt")?;
    let encrypted = hex::decode(&wallet.encrypted_key).context("decode beam wallet ciphertext")?;
    let encrypted =
        EncryptedSymmetricData::from_bytes(&encrypted).context("parse beam wallet ciphertext")?;
    let key = wallet.kdf.derive_key(password, &salt)?;
    let secret_key = symmetric_decrypt(Key::from_slice(&key), &encrypted)
        .map_err(|_| Error::DecryptionFailed)?;
    ensure_wallet_secret_key_matches_address(wallet, &secret_key)?;

    Ok(secret_key)
}

fn ensure_wallet_secret_key_matches_address(
    wallet: &StoredWallet,
    secret_key: &[u8],
) -> Result<()> {
    let stored_address = wallet
        .address
        .parse::<Address>()
        .map_err(|_| Error::InvalidAddress {
            value: wallet.address.clone(),
        })?;
    let derived_address = wallet_address(secret_key)?;

    if stored_address != derived_address {
        return Err(Error::StoredWalletAddressMismatch {
            derived: format!("{derived_address:#x}"),
            stored: wallet.address.clone(),
        });
    }

    Ok(())
}
