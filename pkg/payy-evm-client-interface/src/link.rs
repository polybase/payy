use serde::{Deserialize, Serialize};

use crate::evm::B256;
use crate::operation::IncomingTransfer;

/// Hostless claim link string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimLink {
    /// Link string.
    pub value: String,
}

/// Direct linked-note source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectLinkedNote {
    /// Note.
    pub note: zk_primitives::EvmNote,
    /// Commitment.
    pub commitment: B256,
}

/// Claim-source kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimSourceKind {
    /// Direct recipient-owned note.
    Direct,
    /// Bearer-style ephemeral note.
    Ephemeral,
}

/// Parsed claim link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedClaimLink {
    /// Optional message path segment.
    pub message: Option<String>,
    /// Claim source kind.
    pub claim_source_kind: ClaimSourceKind,
    /// Direct note source.
    pub direct_note: Option<DirectLinkedNote>,
    /// Ephemeral transfer source.
    pub incoming_transfer: Option<IncomingTransfer>,
}
