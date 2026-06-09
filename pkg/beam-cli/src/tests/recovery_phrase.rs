use crate::{
    error::Error,
    keystore::wallet_address,
    recovery_phrase::{
        PRIVATE_KEY_ENTROPY_BYTES, RECOVERY_PHRASE_WORDS, normalize_recovery_phrase,
        private_key_to_recovery_phrase, recovery_phrase_to_private_key,
    },
};

const PRIVATE_KEY: &str = "4f3edf983ac636a65a842ce7c78d9aa706d3b113bce036f6c4d1f06b2d1f6f9d";
const RECOVERY_PHRASE: &str = "execute want toward intact gloom farm head machine treat detect grit evoke honey sudden exclude orchard dad renew crucial this ready moral salmon pave";

#[test]
fn encodes_private_key_bytes_as_payy_compatible_recovery_phrase() {
    let private_key = hex::decode(PRIVATE_KEY).expect("decode private key");
    let phrase = private_key_to_recovery_phrase(&private_key).expect("encode recovery phrase");

    assert_eq!(phrase, RECOVERY_PHRASE);
    assert_eq!(phrase.split_whitespace().count(), RECOVERY_PHRASE_WORDS);
}

#[test]
fn decodes_normalized_recovery_phrase_as_private_key_bytes() {
    let noisy_phrase = RECOVERY_PHRASE
        .split_whitespace()
        .map(str::to_uppercase)
        .collect::<Vec<_>>()
        .join("\n\t");

    assert_eq!(normalize_recovery_phrase(&noisy_phrase), RECOVERY_PHRASE);

    let private_key =
        recovery_phrase_to_private_key(&noisy_phrase).expect("decode recovery phrase");
    assert_eq!(hex::encode(private_key), PRIVATE_KEY);
}

#[test]
fn rejects_recovery_phrase_with_wrong_word_count() {
    let err = recovery_phrase_to_private_key("abandon abandon")
        .expect_err("reject short recovery phrase");

    assert!(matches!(
        err,
        Error::InvalidRecoveryPhraseWordCount {
            expected: RECOVERY_PHRASE_WORDS,
            got: 2,
        }
    ));
}

#[test]
fn rejects_recovery_phrase_with_bad_checksum() {
    let bad_phrase = format!(
        "{} zoo",
        RECOVERY_PHRASE
            .split_whitespace()
            .take(RECOVERY_PHRASE_WORDS - 1)
            .collect::<Vec<_>>()
            .join(" ")
    );
    let err = recovery_phrase_to_private_key(&bad_phrase)
        .expect_err("reject invalid recovery phrase checksum");

    assert!(matches!(err, Error::InvalidRecoveryPhrase));
}

#[test]
fn rejects_recovery_phrase_with_invalid_secp256k1_private_key_entropy() {
    let zero_entropy_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let err =
        recovery_phrase_to_private_key(zero_entropy_phrase).expect_err("reject zero private key");

    assert!(matches!(err, Error::InvalidRecoveryPhrasePrivateKey));
}

#[test]
fn rejects_private_key_entropy_lengths_other_than_32_bytes() {
    let err = private_key_to_recovery_phrase(&[1u8; PRIVATE_KEY_ENTROPY_BYTES - 1])
        .expect_err("reject short private key");

    assert!(matches!(
        err,
        Error::InvalidRecoveryPhraseEntropyLength { length: 31 }
    ));
}

#[test]
fn recovered_phrase_preserves_evm_and_privacy_addresses() {
    let private_key = hex::decode(PRIVATE_KEY).expect("decode private key");
    let phrase = private_key_to_recovery_phrase(&private_key).expect("encode recovery phrase");
    let recovered = recovery_phrase_to_private_key(&phrase).expect("decode recovery phrase");

    assert_eq!(
        wallet_address(&private_key).expect("derive original evm address"),
        wallet_address(&recovered).expect("derive recovered evm address")
    );

    let private_key: [u8; PRIVATE_KEY_ENTROPY_BYTES] =
        private_key.try_into().expect("private key bytes");
    let original_privacy_address =
        payy_evm_client::LocalPrivacySigner::from_evm_private_key(private_key)
            .expect("derive original privacy signer")
            .privacy_address();
    let recovered_privacy_address =
        payy_evm_client::LocalPrivacySigner::from_evm_private_key(recovered)
            .expect("derive recovered privacy signer")
            .privacy_address();

    assert_eq!(original_privacy_address, recovered_privacy_address);
}
