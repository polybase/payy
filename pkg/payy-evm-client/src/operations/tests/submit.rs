use std::sync::Arc;

use element::Element;
use payy_evm_client_interface::{
    DirectSendDelivery, Error, IncomingTransfer, PayyEvmSubmitter, PayyNetworkPreset,
    PrivacyAddress, PrivacyOperationKind, ValidationErrorKind,
};
use zk_primitives::EvmNote;

use super::{MockSubmitter, prepared_with_payload, prepared_with_submitter};

#[tokio::test]
async fn rejects_populated_from_when_submitter_address_differs() {
    let chain_id = PayyNetworkPreset::Dev.config().chain_id;
    let expected_sender = [0x11; 20];
    let submitter = Arc::new(MockSubmitter::new(chain_id, Some([0x22; 20])));
    let prepared = prepared_with_submitter(
        chain_id,
        Some(expected_sender),
        Some(submitter.clone() as Arc<dyn PayyEvmSubmitter>),
    );

    let err = prepared.submit().await.expect_err("submit should reject");

    assert!(matches!(
        err,
        Error::Validation {
            kind: ValidationErrorKind::EvmAccountMismatch
        }
    ));
    assert!(!submitter.sent());
}

#[tokio::test]
async fn rejects_populated_from_when_submitter_address_is_unknown() {
    let chain_id = PayyNetworkPreset::Dev.config().chain_id;
    let submitter = Arc::new(MockSubmitter::new(chain_id, None));
    let prepared = prepared_with_submitter(
        chain_id,
        Some([0x11; 20]),
        Some(submitter.clone() as Arc<dyn PayyEvmSubmitter>),
    );

    let err = prepared.submit().await.expect_err("submit should reject");

    assert!(matches!(
        err,
        Error::Validation {
            kind: ValidationErrorKind::EvmAccountMismatch
        }
    ));
    assert!(!submitter.sent());
}

#[tokio::test]
async fn accepts_matching_populated_from() {
    let chain_id = PayyNetworkPreset::Dev.config().chain_id;
    let sender = [0x11; 20];
    let submitter = Arc::new(MockSubmitter::new(chain_id, Some(sender)));
    let prepared = prepared_with_submitter(
        chain_id,
        Some(sender),
        Some(submitter.clone() as Arc<dyn PayyEvmSubmitter>),
    );

    let result = prepared.submit().await.expect("submit should pass");

    assert_eq!(result.source_tx_hash, [0x99; 32]);
    assert!(submitter.sent());
}

#[tokio::test]
async fn chain_id_mismatch_reports_prepared_call_chain_id() {
    let network_chain_id = PayyNetworkPreset::Dev.config().chain_id;
    let prepared_chain_id = network_chain_id + 1;
    let submitter = Arc::new(MockSubmitter::new(network_chain_id, Some([0x11; 20])));
    let prepared = prepared_with_submitter(
        prepared_chain_id,
        Some([0x11; 20]),
        Some(submitter.clone() as Arc<dyn PayyEvmSubmitter>),
    );

    let err = prepared.submit().await.expect_err("submit should reject");

    assert!(matches!(
        err,
        Error::ChainIdMismatch { expected, actual }
            if expected == prepared_chain_id && actual == network_chain_id
    ));
    assert!(!submitter.sent());
}

#[tokio::test]
async fn transfer_send_submit_hydrates_delivery_source_tx_hash() {
    let chain_id = PayyNetworkPreset::Dev.config().chain_id;
    let submitter = Arc::new(MockSubmitter::new(chain_id, Some([0x11; 20])));
    let mut prepared = prepared_with_payload(
        chain_id,
        None,
        Some(submitter.clone() as Arc<dyn PayyEvmSubmitter>),
        DirectSendDelivery {
            recipient: PrivacyAddress::new([0x22; 32]),
            note: test_note(),
            commitment: [0x33; 32],
            source_tx_hash: None,
            source_bridge_tx_hash: Some([0x44; 32]),
        },
    );
    prepared.result.prepared_call.operation = PrivacyOperationKind::TransferSend;

    let result = prepared.submit().await.expect("submit should pass");

    assert_eq!(result.source_tx_hash, [0x99; 32]);
    assert_eq!(result.payload.source_tx_hash, Some([0x99; 32]));
    assert_eq!(result.payload.source_bridge_tx_hash, Some([0x44; 32]));
    assert!(submitter.sent());
}

#[tokio::test]
async fn transfer_claim_submit_preserves_incoming_transfer_source_tx_hash() {
    let chain_id = PayyNetworkPreset::Dev.config().chain_id;
    let submitter = Arc::new(MockSubmitter::new(chain_id, Some([0x11; 20])));
    let mut prepared = prepared_with_payload(
        chain_id,
        None,
        Some(submitter.clone() as Arc<dyn PayyEvmSubmitter>),
        IncomingTransfer {
            note: test_note(),
            commitment: [0x33; 32],
            ephemeral_private_key: [0x44; 32],
            source_tx_hash: Some([0x55; 32]),
            source_bridge_tx_hash: Some([0x66; 32]),
        },
    );
    prepared.result.prepared_call.operation = PrivacyOperationKind::TransferClaim;

    let result = prepared.submit().await.expect("submit should pass");

    assert_eq!(result.source_tx_hash, [0x99; 32]);
    assert_eq!(result.payload.source_tx_hash, Some([0x55; 32]));
    assert_eq!(result.payload.source_bridge_tx_hash, Some([0x66; 32]));
    assert!(submitter.sent());
}

fn test_note() -> EvmNote {
    EvmNote {
        kind: Element::ONE,
        token: Element::ONE,
        nonce: Element::ZERO,
        psi: Element::ONE,
        owner: Element::ONE,
        value: Element::ONE,
    }
}
