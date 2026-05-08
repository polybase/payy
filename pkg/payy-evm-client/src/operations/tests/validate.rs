use element::Element;
use payy_evm_client_interface::{
    Error, IncomingNote, IncomingNoteStatus, IncomingTransfer, SourceChainPosition,
    ValidationErrorKind,
};
use zk_primitives::EvmNote;

use super::super::validate::{
    IncomingTransferSource, validate_incoming_note, validate_incoming_transfer,
};
use crate::util::element_to_b256;

#[test]
fn direct_incoming_note_rejects_noncanonical_kind() {
    let mut note = valid_note();
    note.kind = Element::new(2);
    let incoming = incoming_note(note);

    let err = validate_incoming_note(&incoming).expect_err("note should reject");

    assert!(matches!(
        err,
        Error::Validation {
            kind: ValidationErrorKind::InvalidIncomingTransfer
        }
    ));
}

#[test]
fn direct_incoming_transfer_rejects_noncanonical_field() {
    let mut note = valid_note();
    note.psi = Element::MODULUS;
    let transfer = incoming_transfer(note);

    let err = validate_incoming_transfer(&transfer, IncomingTransferSource::Direct)
        .expect_err("transfer should reject");

    assert!(matches!(
        err,
        Error::Validation {
            kind: ValidationErrorKind::FieldOutOfRange
        }
    ));
}

#[test]
fn direct_incoming_transfer_rejects_out_of_range_value() {
    let mut note = valid_note();
    let mut value = [0u8; 32];
    value[1] = 1;
    note.value = Element::from_be_bytes(value);
    let transfer = incoming_transfer(note);

    let err = validate_incoming_transfer(&transfer, IncomingTransferSource::Direct)
        .expect_err("transfer should reject");

    assert!(matches!(
        err,
        Error::Validation {
            kind: ValidationErrorKind::ValueOutOfRange
        }
    ));
}

fn incoming_note(note: EvmNote) -> IncomingNote {
    IncomingNote {
        commitment: element_to_b256(note.commitment()),
        nullifier: element_to_b256(note.nullifier()),
        note,
        source_position: SourceChainPosition {
            block_number: 1,
            transaction_index: 0,
            log_index: 0,
        },
        source_tx_hash: [0x11; 32],
        source_bridge_tx_hash: [0x22; 32],
        status: IncomingNoteStatus::Claimable,
    }
}

fn incoming_transfer(note: EvmNote) -> IncomingTransfer {
    IncomingTransfer {
        commitment: element_to_b256(note.commitment()),
        note,
        ephemeral_private_key: [0x33; 32],
        source_tx_hash: None,
        source_bridge_tx_hash: None,
    }
}

fn valid_note() -> EvmNote {
    EvmNote {
        kind: Element::ONE,
        token: Element::ONE,
        nonce: Element::ZERO,
        psi: Element::ONE,
        owner: Element::ONE,
        value: Element::ONE,
    }
}
