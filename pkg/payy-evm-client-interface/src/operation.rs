use serde::{Deserialize, Serialize};

use crate::account::PrivacyAddress;
use crate::evm::{Address, B256, Bytes, PayyEvmTransactionRequest, PayyTransactionReceipt, TxHash};
use crate::note::{IncomingNote, OwnedNote};

/// Privacy operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyOperationKind {
    /// Mint operation.
    Mint,
    /// Burn operation.
    Burn,
    /// Transfer send operation.
    TransferSend,
    /// Transfer claim operation.
    TransferClaim,
}

/// State transition preview for a prepared call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyStatePreview {
    /// Resolved privacy address.
    pub privacy_account: PrivacyAddress,
    /// Token.
    pub token: Address,
    /// Recent root used by the proof.
    pub recent_root: B256,
    /// Non-zero input commitments in circuit order.
    pub input_commitments: Vec<B256>,
    /// Non-zero input nullifiers in circuit order.
    pub input_nullifiers: Vec<B256>,
    /// Non-zero output commitments in circuit order.
    pub output_commitments: Vec<B256>,
}

/// Stable prepared proof and bridge call payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedPrivacyCall {
    /// Operation kind.
    pub operation: PrivacyOperationKind,
    /// EVM chain ID used by proof inputs and transaction commitments.
    pub chain_id: u64,
    /// Bridge transaction request.
    pub bridge_request: PayyEvmTransactionRequest,
    /// Verification key hash.
    pub verification_key_hash: B256,
    /// Proof bytes.
    pub proof: Bytes,
    /// Public inputs.
    pub public_inputs: Vec<B256>,
    /// Transaction commitment.
    pub tx_commitment: B256,
    /// State preview.
    pub state_preview: PrivacyStatePreview,
}

/// Prepared operation wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedOperationResult<TPayload> {
    /// Prepared proof/call payload.
    pub prepared_call: PreparedPrivacyCall,
    /// Operation-specific payload.
    pub payload: TPayload,
}

/// Common view over values that contain a prepared privacy call.
pub trait PreparedPrivacyCallSource {
    /// Return the contained prepared privacy call.
    fn prepared_privacy_call(&self) -> &PreparedPrivacyCall;
}

impl PreparedPrivacyCallSource for PreparedPrivacyCall {
    fn prepared_privacy_call(&self) -> &PreparedPrivacyCall {
        self
    }
}

impl<TPayload> PreparedPrivacyCallSource for PreparedOperationResult<TPayload> {
    fn prepared_privacy_call(&self) -> &PreparedPrivacyCall {
        &self.prepared_call
    }
}

impl<TPayload> PreparedPrivacyCallSource for SubmittedOperationResult<TPayload> {
    fn prepared_privacy_call(&self) -> &PreparedPrivacyCall {
        &self.prepared_call
    }
}

impl<TPayload> PreparedPrivacyCallSource for ConfirmedOperationResult<TPayload> {
    fn prepared_privacy_call(&self) -> &PreparedPrivacyCall {
        &self.prepared_call
    }
}

/// Submitted operation wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmittedOperationResult<TPayload> {
    /// Prepared proof/call payload.
    pub prepared_call: PreparedPrivacyCall,
    /// Operation-specific payload.
    pub payload: TPayload,
    /// Source outer transaction hash.
    pub source_tx_hash: TxHash,
}

/// Confirmed operation wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedOperationResult<TPayload> {
    /// Prepared proof/call payload.
    pub prepared_call: PreparedPrivacyCall,
    /// Operation-specific payload.
    pub payload: TPayload,
    /// Source outer transaction hash.
    pub source_tx_hash: TxHash,
    /// Successful receipt.
    pub receipt: PayyTransactionReceipt,
}

/// Direct-send delivery artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectSendDelivery {
    /// Original recipient privacy address.
    pub recipient: PrivacyAddress,
    /// Recipient note.
    pub note: zk_primitives::EvmNote,
    /// Note commitment.
    pub commitment: B256,
    /// Source transaction hash, when known.
    pub source_tx_hash: Option<TxHash>,
    /// Source bridge transaction hash, when known.
    pub source_bridge_tx_hash: Option<B256>,
}

/// Bearer-style incoming transfer artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomingTransfer {
    /// Received note.
    pub note: zk_primitives::EvmNote,
    /// Note commitment.
    pub commitment: B256,
    /// Ephemeral private key.
    pub ephemeral_private_key: B256,
    /// Source transaction hash, when known.
    pub source_tx_hash: Option<TxHash>,
    /// Source bridge transaction hash, when known.
    pub source_bridge_tx_hash: Option<B256>,
}

/// Manual real input witness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealInputNote {
    /// Owned note metadata.
    pub owned_note: OwnedNote,
    /// Merkle path siblings.
    pub merkle_path: Vec<B256>,
    /// Recent root.
    pub recent_root: B256,
}

/// Manual padding input witness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaddingInputNote {
    /// Recent root.
    pub recent_root: B256,
}

/// Manual input selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ResolvedInputNote {
    /// Real note.
    Real(Box<RealInputNote>),
    /// Padding note.
    Padding(PaddingInputNote),
}

/// Manual claim input override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimResolvedInputs {
    /// Recipient wallet-chain input.
    pub owned_input: ResolvedInputNote,
    /// Incoming note input.
    pub incoming_input: RealInputNote,
}

/// Claim-note payload marker.
pub type ClaimNotePayload = IncomingNote;
