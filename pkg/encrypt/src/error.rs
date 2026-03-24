use crypto_secretbox::aead;

/// Result for encrypt/decrypt
pub type Result<R> = std::result::Result<R, Error>;

/// Error for encrypt/decrypt
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid encrypt data version
    #[error("invalid version, expected {0} got {1}")]
    InvalidVersion(u8, u8),
    /// Data length is not long enough
    #[error("data is not long enough to conver to encryped data, expected {0} got {0}")]
    InvalidDataLength(usize, usize),
    /// Key length is invalid size
    #[error("invalid key length, expected {0} got {1}")]
    InvalidKeyLength(usize, usize),
    /// Encryption failed
    #[error("encrypt failed")]
    EncryptFailed(aead::Error),
    /// Decryption failed
    #[error("decrypt failed")]
    DecryptFailed(aead::Error),
}
