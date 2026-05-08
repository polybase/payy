#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Typed proving boundary for the Payy-EVM client.

use async_trait::async_trait;
use contextful::{FromContextful, InternalError};
use payy_evm_client_interface::{B256, Bytes, PrivacyOperationKind};
use serde::{Deserialize, Serialize};

/// Prover interface error.
#[derive(Debug, thiserror::Error, FromContextful)]
pub enum Error {
    /// The selected circuit is not supported by this prover.
    #[error("[payy-evm-client-prover-interface/error] unsupported circuit: {circuit:?}")]
    UnsupportedCircuit {
        /// Requested circuit.
        circuit: PrivacyCircuit,
    },

    /// Witness bytes could not be decoded for the selected circuit.
    #[error("[payy-evm-client-prover-interface/error] invalid witness for circuit: {circuit:?}")]
    InvalidWitness {
        /// Requested circuit.
        circuit: PrivacyCircuit,
    },

    /// Backend returned malformed proof bytes for the selected circuit.
    #[error(
        "[payy-evm-client-prover-interface/error] invalid proof for circuit: {circuit:?}, kind: {kind:?}"
    )]
    InvalidProof {
        /// Requested circuit.
        circuit: PrivacyCircuit,
        /// Decode failure kind.
        kind: ProofDecodeErrorKind,
    },

    /// Internal implementation failure.
    #[error("[payy-evm-client-prover-interface/error] internal error")]
    Internal(#[from] InternalError),
}

/// Prover result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Supported Payy-EVM privacy circuits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyCircuit {
    /// Mint circuit.
    Mint,
    /// Burn circuit.
    Burn,
    /// Transfer send circuit.
    TransferSend,
    /// Transfer claim circuit.
    TransferClaim,
}

/// Stable proof decode failure kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofDecodeErrorKind {
    /// Public input byte length overflowed while computing the minimum proof size.
    PublicInputLengthOverflow,
    /// Backend proof bytes did not contain all expected public inputs.
    PublicInputsTooShort,
    /// Backend proof bytes were not field-aligned.
    LengthNotMultipleOfField,
}

impl From<PrivacyCircuit> for PrivacyOperationKind {
    fn from(circuit: PrivacyCircuit) -> Self {
        match circuit {
            PrivacyCircuit::Mint => Self::Mint,
            PrivacyCircuit::Burn => Self::Burn,
            PrivacyCircuit::TransferSend => Self::TransferSend,
            PrivacyCircuit::TransferClaim => Self::TransferClaim,
        }
    }
}

/// Serialized circuit witness request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProveRequest {
    /// Circuit to prove.
    pub circuit: PrivacyCircuit,
    /// Serialized witness bytes for the selected circuit.
    pub witness: Bytes,
}

/// Prover output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProveOutput {
    /// Verification key hash.
    pub verification_key_hash: B256,
    /// Proof bytes.
    pub proof: Bytes,
    /// Canonical public inputs in bridge order.
    pub public_inputs: Vec<B256>,
}

/// Payy-EVM privacy prover.
#[async_trait]
pub trait PrivacyProver: Send + Sync {
    /// Prove a serialized witness for the selected circuit.
    async fn prove(&self, request: ProveRequest) -> Result<ProveOutput>;
}
