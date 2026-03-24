use crate::{Error, NONCE_SIZE, Result, VERSION};
use crypto_secretbox::{
    Key, Nonce, XSalsa20Poly1305,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};

/// Encrypt symmetric data
#[derive(Debug)]
pub struct EncryptedSymmetricData {
    /// Nonce for randomness (every encryption should have a new randomness to protect the
    /// security properties of the underlying key)
    pub nonce: Vec<u8>,
    /// Encrypted data bytes
    pub ciphertext: Vec<u8>,
}

impl EncryptedSymmetricData {
    /// Converts encrypted data into compact byte form
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let combined = Vec::with_capacity(1 + self.nonce.len() + self.ciphertext.len());
        let mut result = combined;

        // Add version
        result.push(VERSION);

        // Add nonce
        result.extend_from_slice(&self.nonce);

        // Add ciphertext
        result.extend_from_slice(&self.ciphertext);

        result
    }

    /// Restores encrypted data from compact byte form
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Check if the data has at least enough bytes for version and nonce
        if bytes.len() < 1 + NONCE_SIZE {
            Err(Error::InvalidDataLength(1 + NONCE_SIZE, bytes.len()))?;
        }

        // Read version
        let version = bytes[0];

        // Assert version is 1
        if version != VERSION {
            return Err(Error::InvalidVersion(VERSION, version))?;
        }

        // Extract nonce (bytes 1 to 1+NONCE_SIZE)
        let nonce = bytes[1..=NONCE_SIZE].to_vec();

        // Extract ciphertext (remaining bytes)
        let ciphertext = bytes[1 + NONCE_SIZE..].to_vec();

        Ok(Self { nonce, ciphertext })
    }
}

/// Encrypts data symmetrically using XSalsa20Poly1305
pub fn symmetric_encrypt(key: &Key, data: &[u8]) -> Result<EncryptedSymmetricData> {
    // Generate random nonce
    let nonce = XSalsa20Poly1305::generate_nonce(&mut OsRng);

    // Encrypt with XSalsa20Poly1305
    // let key = Key::from_slice(key_bytes);
    let cipher = XSalsa20Poly1305::new(key);
    let ciphertext = cipher
        .encrypt(&nonce, data.as_ref())
        .map_err(Error::EncryptFailed)?;

    Ok(EncryptedSymmetricData {
        nonce: nonce.to_vec(),
        ciphertext,
    })
}

/// Decrypts data symmetrically using XSalsa20Poly1305
pub fn symmetric_decrypt(key: &Key, encrypted: &EncryptedSymmetricData) -> Result<Vec<u8>> {
    // Convert nonce
    let nonce = Nonce::from_slice(&encrypted.nonce);

    // Decrypt with XSalsa20Poly1305
    let cipher = XSalsa20Poly1305::new(key);
    let plaintext = cipher
        .decrypt(nonce, encrypted.ciphertext.as_slice())
        .map_err(Error::DecryptFailed)?;

    Ok(plaintext)
}

/// Generates a new random symmetric key
pub fn generate_symmetric_key() -> Key {
    // Generate a random key for XSalsa20Poly1305
    XSalsa20Poly1305::generate_key(&mut OsRng)
}
