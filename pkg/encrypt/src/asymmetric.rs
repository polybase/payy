use crate::{EPHEMERAL_PUBLIC_KEY_SIZE, Error, NONCE_SIZE, Result, VERSION, util::to_array_32};
use crypto_secretbox::{
    Key, Nonce, XSalsa20Poly1305,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

/// Asymmetric encrypted data ready for storage/exchange. Can be decrypted with only the corrosponding
/// public key
#[derive(Debug)]
pub struct EncryptedAsymmetricData {
    /// Nonce required for randomness
    pub nonce: Vec<u8>,
    /// Emphemeral public key from the other ser
    pub ephemeral_public_key: Vec<u8>,
    /// Cipher text of encrypted data
    pub ciphertext: Vec<u8>,
}

impl EncryptedAsymmetricData {
    /// Converts encryption into the compact data format (i.e. to be stored)
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let combined = Vec::with_capacity(
            1 + self.nonce.len() + self.ephemeral_public_key.len() + self.ciphertext.len(),
        );
        let mut result = combined;

        // Add version
        result.push(VERSION);

        // Add nonce
        result.extend_from_slice(&self.nonce);

        // Add ephemeral public key
        result.extend_from_slice(&self.ephemeral_public_key);

        // Add ciphertext
        result.extend_from_slice(&self.ciphertext);

        result
    }

    /// Creates new asymmetric data struct from bytes (i.e. from storage)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Check if the data has at least enough bytes for version, nonce, and ephemeral public key
        if bytes.len() < 1 + NONCE_SIZE + EPHEMERAL_PUBLIC_KEY_SIZE {
            Err(Error::InvalidDataLength(
                1 + NONCE_SIZE + EPHEMERAL_PUBLIC_KEY_SIZE,
                bytes.len(),
            ))?;
        }

        // Read version
        let version = bytes[0];

        // Assert version is 1
        if version != VERSION {
            return Err(Error::InvalidVersion(VERSION, version));
        }

        // Extract nonce (bytes 1 to 1+NONCE_SIZE)
        let nonce = bytes[1..=NONCE_SIZE].to_vec();

        // Extract ephemeral public key (bytes 1+NONCE_SIZE to 1+NONCE_SIZE+EPHEMERAL_PUBLIC_KEY_SIZE)
        let ephemeral_public_key =
            bytes[1 + NONCE_SIZE..1 + NONCE_SIZE + EPHEMERAL_PUBLIC_KEY_SIZE].to_vec();

        // Extract ciphertext (remaining bytes)
        let ciphertext = bytes[1 + NONCE_SIZE + EPHEMERAL_PUBLIC_KEY_SIZE..].to_vec();

        Ok(Self {
            nonce,
            ephemeral_public_key,
            ciphertext,
        })
    }
}

/// Encrypts data asymmetrically using X25519-XSalsa20-Poly1305
pub fn asymmetric_encrypt(public_key: &PublicKey, data: &[u8]) -> Result<EncryptedAsymmetricData> {
    // Generate ephemeral keypair
    let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral_secret);

    // Perform Diffie-Hellman
    let shared_secret = ephemeral_secret.diffie_hellman(public_key);

    // Generate random nonce
    let nonce = XSalsa20Poly1305::generate_nonce(&mut OsRng);

    // Encrypt with XSalsa20Poly1305
    let key = Key::from_slice(shared_secret.as_bytes());
    let cipher = XSalsa20Poly1305::new(key);
    let ciphertext = cipher
        .encrypt(&nonce, data.as_ref())
        .map_err(Error::EncryptFailed)?;

    Ok(EncryptedAsymmetricData {
        nonce: nonce.to_vec(),
        ephemeral_public_key: ephemeral_public.as_bytes().to_vec(),
        ciphertext,
    })
}

/// Decrypts data asymmetrically using X25519-XSalsa20-Poly1305
pub fn asymmetric_decrypt(
    secret_key: &StaticSecret,
    encrypted: &EncryptedAsymmetricData,
) -> Result<Vec<u8>> {
    // Convert sender public key
    let sender_public_array = to_array_32(&encrypted.ephemeral_public_key)?;
    let sender_public = PublicKey::from(sender_public_array);

    // Perform Diffie-Hellman
    let shared_secret = secret_key.diffie_hellman(&sender_public);

    // Convert nonce
    let nonce = Nonce::from_slice(&encrypted.nonce);

    // Decrypt with XSalsa20Poly1305
    let key = Key::from_slice(shared_secret.as_bytes());
    let cipher = XSalsa20Poly1305::new(key);
    let plaintext = cipher
        .decrypt(nonce, encrypted.ciphertext.as_slice())
        .map_err(Error::DecryptFailed)?;

    Ok(plaintext)
}
