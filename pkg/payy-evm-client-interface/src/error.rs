use contextful::{FromContextful, InternalError};
use serde::{Deserialize, Serialize};

use crate::evm::B256;

/// Stable interface error type for Payy EVM client surfaces.
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize, FromContextful)]
#[serde(tag = "code", content = "data", rename_all = "snake_case")]
pub enum Error {
    /// The configured EVM adapter is connected to the wrong chain.
    #[error(
        "[payy-evm-client-interface/error] chain id mismatch: expected {expected}, got {actual}"
    )]
    ChainIdMismatch {
        /// Configured chain ID.
        expected: u64,
        /// Adapter-reported chain ID.
        actual: u64,
    },

    /// A required capability was not configured.
    #[error("[payy-evm-client-interface/error] missing capability: {capability}")]
    MissingCapability {
        /// Missing capability name.
        capability: &'static str,
    },

    /// Caller-supplied data failed local validation.
    #[error("[payy-evm-client-interface/error] validation failed: {kind}")]
    Validation {
        /// Stable validation failure kind.
        kind: ValidationErrorKind,
    },

    /// A structurally valid claim source has not been published on-chain yet.
    #[error("[payy-evm-client-interface/error] commitment not found: {commitment:?}")]
    CommitmentNotFound {
        /// Unresolved commitment value.
        commitment: B256,
    },

    /// Waiting for a transaction receipt timed out.
    #[error(
        "[payy-evm-client-interface/error] receipt wait timed out for {hash:?} after {timeout_ms}ms"
    )]
    ReceiptTimeout {
        /// Transaction hash being polled.
        hash: B256,
        /// Receipt wait timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Local signing command failed.
    #[error("[payy-evm-client-interface/error] signer command failed: {message}")]
    SignerCommandFailed {
        /// Error message returned by the signer command.
        message: String,
    },

    /// Local signing command returned an unexpected response variant.
    #[error("[payy-evm-client-interface/error] unexpected signer response variant: {variant}")]
    UnexpectedSignerResponse {
        /// Response variant returned by the signer command.
        variant: String,
    },

    /// Local signing command returned a malformed byte field.
    #[error(
        "[payy-evm-client-interface/error] signer response field {field} has invalid length: {length}"
    )]
    InvalidSignerResponseLength {
        /// Malformed response field.
        field: SignerResponseField,
        /// Actual byte length returned by the signer command.
        length: usize,
    },

    /// Transaction receipt reported a reverted transaction.
    #[error("[payy-evm-client-interface/error] transaction reverted")]
    TransactionReverted,

    /// Internal implementation failure.
    #[error("[payy-evm-client-interface/error] internal error")]
    Internal(#[from] InternalError),
}

/// Stable signer response field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignerResponseField {
    /// Schnorr signature `s` field.
    S,
    /// Schnorr signature `e` field.
    E,
}

impl std::fmt::Display for SignerResponseField {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

/// Stable local validation failure kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationErrorKind {
    /// Amount must be non-zero.
    AmountZero,
    /// EVM recipient must not be the zero address.
    EvmRecipientZero,
    /// Selected privacy account does not own the note.
    PrivacyAccountMismatch,
    /// Selected EVM account cannot submit the transaction.
    EvmAccountMismatch,
    /// A note commitment did not match the note fields.
    CommitmentMismatch,
    /// A note nullifier did not match the note fields.
    NullifierMismatch,
    /// A first note nonce hash did not match the note fields.
    NonceHashMismatch,
    /// A note is already spent.
    NoteSpent,
    /// A spend operation requires an owned input note but none is available.
    MissingOwnedNote,
    /// The selected owned note does not contain enough value.
    InsufficientBalance,
    /// A checkpoint belongs to a different account or token.
    CheckpointMismatch,
    /// A Merkle witness path has an invalid shape.
    MerklePathInvalid,
    /// A received-note transfer violates incoming-transfer invariants.
    InvalidIncomingTransfer,
    /// Claim link payload is malformed or unsupported.
    InvalidClaimLink,
    /// Claim link version is not supported by this SDK.
    UnsupportedClaimLinkVersion,
    /// A field element encoding is out of canonical range.
    FieldOutOfRange,
    /// A value exceeds the protocol 240-bit value bound.
    ValueOutOfRange,
    /// Ephemeral private key does not derive the carried owner.
    EphemeralKeyMismatch,
    /// Privacy address bytes are not a canonical compressed Grumpkin public key.
    InvalidPrivacyAddress,
    /// A supplied prefix does not match the selected privacy account.
    PrefixMismatch,
    /// A privacy signer returned a malformed owner signature.
    InvalidOwnerSignature,
}

impl std::fmt::Display for ValidationErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

/// Interface result alias.
pub type Result<T> = std::result::Result<T, Error>;
