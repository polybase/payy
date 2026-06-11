use bip39::{Language, Mnemonic};
use secp256k1::SecretKey;

use crate::error::{Error, Result};

pub const PRIVATE_KEY_ENTROPY_BYTES: usize = 32;
pub const RECOVERY_PHRASE_WORDS: usize = 24;

pub fn normalize_recovery_phrase(phrase: &str) -> String {
    phrase
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Encodes secp256k1 private key bytes as BIP39 entropy.
/// This does not use seed or HD derivation.
pub fn private_key_to_recovery_phrase(private_key: &[u8]) -> Result<String> {
    let private_key = private_key_entropy(private_key)?;
    validate_private_key_bytes(&private_key)?;

    let mnemonic = Mnemonic::from_entropy_in(Language::English, &private_key)
        .map_err(|_| Error::InvalidRecoveryPhrase)?;
    let phrase = mnemonic.to_string();

    debug_assert_eq!(phrase.split_whitespace().count(), RECOVERY_PHRASE_WORDS);
    Ok(phrase)
}

/// Decodes BIP39 mnemonic entropy as secp256k1 private key bytes.
/// This does not use seed or HD derivation.
pub fn recovery_phrase_to_private_key(phrase: &str) -> Result<[u8; PRIVATE_KEY_ENTROPY_BYTES]> {
    let normalized = normalize_recovery_phrase(phrase);
    ensure_recovery_phrase_word_count(&normalized)?;

    let mnemonic = Mnemonic::parse_in(Language::English, &normalized)
        .map_err(|_| Error::InvalidRecoveryPhrase)?;

    let private_key: [u8; PRIVATE_KEY_ENTROPY_BYTES] =
        mnemonic
            .to_entropy()
            .try_into()
            .map_err(
                |entropy: Vec<u8>| Error::InvalidRecoveryPhraseEntropyLength {
                    length: entropy.len(),
                },
            )?;
    validate_recovery_phrase_private_key_bytes(&private_key)?;

    Ok(private_key)
}

fn private_key_entropy(private_key: &[u8]) -> Result<[u8; PRIVATE_KEY_ENTROPY_BYTES]> {
    private_key
        .try_into()
        .map_err(|_| Error::InvalidRecoveryPhraseEntropyLength {
            length: private_key.len(),
        })
}

fn ensure_recovery_phrase_word_count(phrase: &str) -> Result<()> {
    let word_count = phrase.split_whitespace().count();
    if word_count != RECOVERY_PHRASE_WORDS {
        return Err(Error::InvalidRecoveryPhraseWordCount {
            expected: RECOVERY_PHRASE_WORDS,
            got: word_count,
        });
    }

    Ok(())
}

fn validate_private_key_bytes(private_key: &[u8; PRIVATE_KEY_ENTROPY_BYTES]) -> Result<()> {
    let _ = SecretKey::from_slice(private_key).map_err(|_| Error::InvalidPrivateKey)?;
    Ok(())
}

fn validate_recovery_phrase_private_key_bytes(
    private_key: &[u8; PRIVATE_KEY_ENTROPY_BYTES],
) -> Result<()> {
    let _ =
        SecretKey::from_slice(private_key).map_err(|_| Error::InvalidRecoveryPhrasePrivateKey)?;
    Ok(())
}
