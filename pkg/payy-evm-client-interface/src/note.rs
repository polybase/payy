use element::Element;
use serde::{Deserialize, Serialize};
use zk_primitives::EvmNote;

use crate::account::PrivacyAddress;
use crate::evm::{Address, B256, TxHash};

/// PrivacyBridge transaction data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxnData {
    /// Verification key hash.
    pub verification_key_hash: B256,
    /// Sender encrypted note words.
    pub sender_encrypted_note: [B256; 5],
    /// Recipient encrypted note words.
    pub recipient_encrypted_note: [B256; 5],
    /// Sender chain encrypted key words.
    pub sender_chain_encrypted_key: [B256; 3],
    /// Recipient chain encrypted key words.
    pub recipient_chain_encrypted_key: [B256; 3],
    /// User encrypted key words.
    pub user_encrypted_key: [B256; 4],
    /// Recipient encrypted key words.
    pub recipient_encrypted_key: [B256; 4],
    /// Bridge memo.
    pub memo: B256,
}

/// Resolved wallet-chain note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedNote {
    /// Canonical note.
    pub note: EvmNote,
    /// Note commitment.
    pub commitment: B256,
    /// Spend nullifier.
    pub nullifier: B256,
    /// Nonce hash used to discover this note.
    pub nonce_hash: B256,
    /// Source block, when known.
    pub source_block: Option<u64>,
    /// Source transaction hash, when known.
    pub source_tx_hash: Option<TxHash>,
    /// Source bridge transaction hash, when known.
    pub source_bridge_tx_hash: Option<B256>,
}

/// Checked-block owned note state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedNoteState {
    /// Resolved privacy address.
    pub privacy_account: PrivacyAddress,
    /// Token address.
    pub token: Address,
    /// Latest owned note.
    pub owned_note: Option<OwnedNote>,
    /// Highest checked block.
    pub checked_block: u64,
}

/// Wallet-facing private balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateBalance {
    /// Resolved privacy address.
    pub privacy_account: PrivacyAddress,
    /// Token address.
    pub token: Address,
    /// Spendable value.
    pub spendable: Element,
}

/// Balance read result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateBalanceState {
    /// Current balance, if an owned note exists.
    pub balance: Option<PrivateBalance>,
    /// Underlying owned-note state.
    pub owned_note_state: OwnedNoteState,
}

/// Source log position for incoming notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceChainPosition {
    /// Source block number.
    pub block_number: u64,
    /// Source transaction index.
    pub transaction_index: u64,
    /// Source log index.
    pub log_index: u64,
}

/// Incoming-note spend status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncomingNoteStatus {
    /// Note is unspent and claimable.
    Claimable,
    /// Note is already spent.
    Spent,
}

/// Discovered on-chain incoming note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomingNote {
    /// Canonical note.
    pub note: EvmNote,
    /// Note commitment.
    pub commitment: B256,
    /// Note nullifier.
    pub nullifier: B256,
    /// Source chain position.
    pub source_position: SourceChainPosition,
    /// Source outer transaction hash.
    pub source_tx_hash: TxHash,
    /// Source bridge transaction hash.
    pub source_bridge_tx_hash: B256,
    /// Current spend status.
    pub status: IncomingNoteStatus,
}
