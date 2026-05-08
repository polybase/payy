use element::Element;
use serde::{Deserialize, Serialize};
use zk_primitives::EvmNote;

use crate::Result;
use crate::account::{PrivacyAccount, PrivacyAddress};
use crate::evm::B256;
use crate::note::TxnData;

/// Owner signature bundle for Grumpkin/Schnorr spend authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerSignature {
    /// Barretenberg-compatible Schnorr signature bytes.
    pub signature: Vec<u8>,
    /// Public key x-coordinate.
    pub public_key_x: Element,
    /// Public key y-coordinate.
    pub public_key_y: Element,
}

/// One-time keypair used by bearer-style sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EphemeralKeyPair {
    /// One-time private key bytes.
    pub private_key: B256,
    /// Derived one-time privacy address.
    pub privacy_address: PrivacyAddress,
}

/// Privacy signer contract.
pub trait PrivacySigner: Send + Sync {
    /// Return controlled account selectors in stable signer-defined order.
    fn accounts(&self) -> Result<Vec<PrivacyAccount>>;
    /// Sign the canonical transaction commitment.
    fn sign_tx_commitment(
        &self,
        privacy_account: PrivacyAccount,
        tx_commitment: B256,
    ) -> Result<OwnerSignature>;
    /// Decrypt sender-facing note payload.
    fn decrypt_sender_note(
        &self,
        privacy_account: PrivacyAccount,
        txn_data: TxnData,
    ) -> Result<Option<EvmNote>>;
    /// Decrypt recipient-facing note payload.
    fn decrypt_recipient_note(
        &self,
        privacy_account: PrivacyAccount,
        txn_data: TxnData,
    ) -> Result<Option<EvmNote>>;
    /// Generate a one-time keypair.
    fn generate_ephemeral_key(&self) -> Result<EphemeralKeyPair>;
}
