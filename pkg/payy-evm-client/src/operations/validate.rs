use element::Element;
use payy_evm_client_interface::{
    IncomingNote, IncomingNoteStatus, IncomingTransfer, Result, ValidationErrorKind,
};
use zk_primitives::EvmNote;

use super::proof::ensure_u240;
use crate::client::PrivacyNamespace;
use crate::links::ephemeral_owner;
use crate::util::{element_to_b256, ensure_amount_non_zero};

#[derive(Clone, Copy)]
pub(super) enum IncomingTransferSource {
    Direct,
    Link,
}

pub(super) fn validate_incoming_note(note: &IncomingNote) -> Result<()> {
    if note.status == IncomingNoteStatus::Spent {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::NoteSpent,
        });
    }
    validate_received_note_shape(note.note, ValidationErrorKind::InvalidIncomingTransfer)?;
    if element_to_b256(note.note.commitment()) != note.commitment {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::CommitmentMismatch,
        });
    }
    if element_to_b256(note.note.nullifier()) != note.nullifier {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::NullifierMismatch,
        });
    }
    Ok(())
}

pub(super) async fn validate_claim_publication(
    client: &PrivacyNamespace,
    note: EvmNote,
) -> Result<()> {
    let commitment = element_to_b256(note.commitment());
    if client
        .bridge()
        .get_txn_hash_by_commitment(commitment)
        .await?
        .is_none()
    {
        return Err(payy_evm_client_interface::Error::CommitmentNotFound { commitment });
    }
    if client
        .bridge()
        .element_exists(element_to_b256(note.nullifier()))
        .await?
    {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::NoteSpent,
        });
    }
    Ok(())
}

pub(super) fn validate_incoming_transfer(
    transfer: &IncomingTransfer,
    source: IncomingTransferSource,
) -> Result<()> {
    validate_received_note_shape(transfer.note, incoming_transfer_error_kind(source))?;
    if element_to_b256(transfer.note.commitment()) != transfer.commitment {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::CommitmentMismatch,
        });
    }
    if ephemeral_owner(&transfer.ephemeral_private_key)? != transfer.note.owner {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: match source {
                IncomingTransferSource::Direct => ValidationErrorKind::EphemeralKeyMismatch,
                IncomingTransferSource::Link => ValidationErrorKind::InvalidClaimLink,
            },
        });
    }
    Ok(())
}

fn validate_received_note_shape(note: EvmNote, shape_kind: ValidationErrorKind) -> Result<()> {
    if note.kind != Element::ONE {
        return Err(payy_evm_client_interface::Error::Validation { kind: shape_kind });
    }
    if note.token >= Element::MODULUS
        || note.nonce >= Element::MODULUS
        || note.psi >= Element::MODULUS
        || note.owner >= Element::MODULUS
    {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::FieldOutOfRange,
        });
    }
    if !is_address_field(note.token) || note.nonce != Element::ZERO {
        return Err(payy_evm_client_interface::Error::Validation { kind: shape_kind });
    }
    ensure_u240(note.value)?;
    ensure_amount_non_zero(note.value)?;
    Ok(())
}

fn is_address_field(value: Element) -> bool {
    value.to_be_bytes()[..12].iter().all(|byte| *byte == 0)
}

fn incoming_transfer_error_kind(source: IncomingTransferSource) -> ValidationErrorKind {
    match source {
        IncomingTransferSource::Direct => ValidationErrorKind::InvalidIncomingTransfer,
        IncomingTransferSource::Link => ValidationErrorKind::InvalidClaimLink,
    }
}
