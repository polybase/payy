use borsh::{BorshDeserialize, BorshSerialize};
use element::Element;
use primitives::serde::{deserialize_base64, serialize_base64};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use crate::bytes_to_elements;

/// Default proof byte length for agg_agg and related proofs.
pub const PROOF_BYTES_LEN: usize = 508 * 32;

/// Default proof byte length for oracle/agg_final proofs.
pub const ORACLE_PROOF_BYTES_LEN: usize = 330 * 32;

/// Raw proof bytes for zk circuit proofs (without public inputs).
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct ProofBytes(
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    #[cfg_attr(feature = "ts-rs", ts(as = "String"))]
    pub Vec<u8>,
);

impl Default for ProofBytes {
    fn default() -> Self {
        Self(vec![0u8; PROOF_BYTES_LEN])
    }
}

impl ProofBytes {
    /// Convert proof bytes into field elements.
    #[must_use]
    pub fn to_fields(&self) -> Vec<Element> {
        bytes_to_elements(&self.0)
    }
}

/// Raw oracle proof bytes for zk circuit proofs (without public inputs).
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct OracleProofBytes(
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    pub Vec<u8>,
);

impl Default for OracleProofBytes {
    fn default() -> Self {
        Self(vec![0u8; ORACLE_PROOF_BYTES_LEN])
    }
}

impl OracleProofBytes {
    /// Convert proof bytes into field elements.
    #[must_use]
    pub fn to_fields(&self) -> Vec<Element> {
        bytes_to_elements(&self.0)
    }
}
