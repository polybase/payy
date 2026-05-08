// lint-long-file-override allow-max-lines=300
use std::sync::Arc;

use async_trait::async_trait;
use element::Element;
use payy_evm_client_interface::{
    B256, EphemeralKeyPair, Error, OwnedNote, OwnedNoteState, OwnerSignature, PayyEvmCallRequest,
    PayyEvmLog, PayyEvmLogFilter, PayyEvmReadClient, PayyNetworkPreset, PayyTransactionReceipt,
    PayyWaitForTransactionReceiptArgs, PrivacyAccount, PrivacyAddress, PrivacySigner,
    PrivacySignerAccount, Result, TxHash, TxnData, ValidationErrorKind,
};
use zk_primitives::EvmNote;

use crate::util::element_to_b256;
use crate::{BaseClient, IncomingListParams, LocalPrivacySigner};

struct MockReadClient {
    chain_id: u64,
}

#[async_trait]
impl PayyEvmReadClient for MockReadClient {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.chain_id)
    }

    async fn get_block_number(&self) -> Result<u64> {
        Ok(11)
    }

    async fn read_contract(&self, _request: PayyEvmCallRequest) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    async fn get_logs(&self, _filter: PayyEvmLogFilter) -> Result<Vec<PayyEvmLog>> {
        Ok(Vec::new())
    }

    async fn get_transaction_receipt(
        &self,
        _hash: TxHash,
    ) -> Result<Option<PayyTransactionReceipt>> {
        Ok(None)
    }

    async fn wait_for_transaction_receipt(
        &self,
        _args: PayyWaitForTransactionReceiptArgs,
    ) -> Result<PayyTransactionReceipt> {
        Err(Error::MissingCapability {
            capability: "receipt",
        })
    }
}

struct IncomingReadClient {
    chain_id: u64,
    logs: Vec<PayyEvmLog>,
}

#[async_trait]
impl PayyEvmReadClient for IncomingReadClient {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.chain_id)
    }

    async fn get_block_number(&self) -> Result<u64> {
        Ok(11)
    }

    async fn read_contract(&self, _request: PayyEvmCallRequest) -> Result<Vec<u8>> {
        Ok(vec![0u8; 32 * 26])
    }

    async fn get_logs(&self, _filter: PayyEvmLogFilter) -> Result<Vec<PayyEvmLog>> {
        Ok(self.logs.clone())
    }

    async fn get_transaction_receipt(
        &self,
        _hash: TxHash,
    ) -> Result<Option<PayyTransactionReceipt>> {
        Ok(None)
    }

    async fn wait_for_transaction_receipt(
        &self,
        _args: PayyWaitForTransactionReceiptArgs,
    ) -> Result<PayyTransactionReceipt> {
        Err(Error::MissingCapability {
            capability: "receipt",
        })
    }
}

struct StaticPrivacySigner {
    note: EvmNote,
}

impl PrivacySigner for StaticPrivacySigner {
    fn accounts(&self) -> Result<Vec<PrivacyAccount>> {
        Ok(Vec::new())
    }

    fn sign_tx_commitment(
        &self,
        _privacy_account: PrivacyAccount,
        _tx_commitment: B256,
    ) -> Result<OwnerSignature> {
        Err(Error::MissingCapability {
            capability: "sign_tx_commitment",
        })
    }

    fn decrypt_sender_note(
        &self,
        _privacy_account: PrivacyAccount,
        _txn_data: TxnData,
    ) -> Result<Option<EvmNote>> {
        Ok(None)
    }

    fn decrypt_recipient_note(
        &self,
        _privacy_account: PrivacyAccount,
        _txn_data: TxnData,
    ) -> Result<Option<EvmNote>> {
        Ok(Some(self.note))
    }

    fn generate_ephemeral_key(&self) -> Result<EphemeralKeyPair> {
        Err(Error::MissingCapability {
            capability: "ephemeral_key",
        })
    }
}

#[test]
fn set_checkpoint_rejects_source_block_after_checked_block() {
    let config = PayyNetworkPreset::Dev.config();
    let read_client = Arc::new(MockReadClient {
        chain_id: config.chain_id,
    });
    let client = BaseClient::builder(config, read_client).build();
    let note = EvmNote {
        kind: Element::ONE,
        token: Element::ONE,
        nonce: Element::ZERO,
        psi: Element::ONE,
        owner: Element::ONE,
        value: Element::ONE,
    };
    let err = client
        .with_grumpkin_private_key([0x11u8; 32])
        .unwrap()
        .privacy()
        .set_checkpoint(OwnedNoteState {
            privacy_account: PrivacyAddress::new([0x05; 32]),
            token: [0x06; 20],
            owned_note: Some(OwnedNote {
                note,
                commitment: element_to_b256(note.commitment()),
                nullifier: element_to_b256(note.nullifier()),
                nonce_hash: [0x07; 32],
                source_block: Some(12),
                source_tx_hash: None,
                source_bridge_tx_hash: None,
            }),
            checked_block: 11,
        })
        .unwrap_err();

    assert!(matches!(
        err,
        Error::Validation {
            kind: ValidationErrorKind::CheckpointMismatch
        }
    ));
}

#[test]
fn set_checkpoint_rejects_first_note_nonce_hash_mismatch() {
    let config = PayyNetworkPreset::Dev.config();
    let read_client = Arc::new(MockReadClient {
        chain_id: config.chain_id,
    });
    let client = BaseClient::builder(config, read_client).build();
    let note = EvmNote {
        kind: Element::ONE,
        token: Element::ONE,
        nonce: Element::ZERO,
        psi: Element::ONE,
        owner: Element::ONE,
        value: Element::ONE,
    };
    let err = client
        .with_grumpkin_private_key([0x11u8; 32])
        .unwrap()
        .privacy()
        .set_checkpoint(OwnedNoteState {
            privacy_account: PrivacyAddress::new([0x05; 32]),
            token: [0x06; 20],
            owned_note: Some(OwnedNote {
                note,
                commitment: element_to_b256(note.commitment()),
                nullifier: element_to_b256(note.nullifier()),
                nonce_hash: [0x07; 32],
                source_block: None,
                source_tx_hash: None,
                source_bridge_tx_hash: None,
            }),
            checked_block: 11,
        })
        .unwrap_err();

    assert!(matches!(
        err,
        Error::Validation {
            kind: ValidationErrorKind::NonceHashMismatch
        }
    ));
}

#[tokio::test]
async fn incoming_list_skips_decrypted_note_for_different_owner() {
    let config = PayyNetworkPreset::Dev.config();
    let address = LocalPrivacySigner::from_grumpkin_private_key([0x11u8; 32])
        .unwrap()
        .privacy_address();
    let owner = address.owner().unwrap();
    let wrong_owner = if owner == Element::ONE {
        Element::from(2u64)
    } else {
        Element::ONE
    };
    let note = EvmNote {
        kind: Element::ONE,
        token: Element::ONE,
        nonce: Element::ZERO,
        psi: Element::ONE,
        owner: wrong_owner,
        value: Element::ONE,
    };
    let read_client = Arc::new(IncomingReadClient {
        chain_id: config.chain_id,
        logs: vec![PayyEvmLog {
            address: config.privacy_bridge,
            data: Vec::new(),
            topics: Vec::new(),
            block_number: 4,
            transaction_index: 0,
            log_index: 0,
            transaction_hash: [0x22; 32],
        }],
    });
    let signer = Arc::new(StaticPrivacySigner { note });
    let account = PrivacyAccount::Signer(PrivacySignerAccount {
        privacy_address: address,
        signer: signer.clone(),
    });
    let client = BaseClient::builder(config, read_client)
        .build()
        .privacy_signer(signer)
        .privacy();

    let incoming = client
        .incoming()
        .list(IncomingListParams {
            privacy_account: account,
            privacy_address_prefix: None,
            from_block: 0,
            to_block: Some(11),
            include_spent: true,
            poll_interval_ms: None,
        })
        .await
        .unwrap();

    assert!(incoming.is_empty());
}
