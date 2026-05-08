use element::Element;
use ethnum::U256;
use payy_evm_client_interface::{B256, Result, ValidationErrorKind};
use zk_primitives::{
    EvmNote, address_to_field, encrypted_key_hash, first_nonce_hash, next_nonce_hash,
    receive_prefix, tx_commitment, user_encrypted_key_hash,
};

use super::proof::{OwnedSpendInput, SpendInput};
use crate::client::PrivacyNamespace;
use crate::util::address_to_element;

pub(super) fn ensure_u240(value: Element) -> Result<()> {
    if value.to_u256() >= (U256::ONE << 240) {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::ValueOutOfRange,
        });
    }
    Ok(())
}

pub(super) fn ensure_balance_at_least(input: EvmNote, amount: Element) -> Result<()> {
    if input.value.to_u256() < amount.to_u256() {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::InsufficientBalance,
        });
    }
    Ok(())
}

pub(super) fn output_nonce_hash(input: EvmNote, output: EvmNote) -> Element {
    if input.kind.is_zero() {
        first_nonce_hash(output.kind, output.token, output.owner)
    } else {
        next_nonce_hash(
            output.kind,
            output.token,
            output.owner,
            output.nonce,
            input.psi,
        )
    }
}

pub(super) fn mint_tx_commitment(
    client: &PrivacyNamespace,
    input: &OwnedSpendInput,
    output_note: EvmNote,
    mint_from: payy_evm_client_interface::Address,
    user_key: [B256; 4],
) -> Element {
    tx_commitment(
        Element::from(client.inner.network.chain_id),
        address_to_element(client.inner.network.privacy_bridge),
        input.commitment(),
        Element::ZERO,
        output_note.commitment(),
        Element::ZERO,
        Element::ZERO,
        address_to_field(mint_from),
        user_encrypted_key_hash(&user_key),
        Element::ZERO,
        Element::ZERO,
    )
}

pub(super) fn burn_tx_commitment(
    client: &PrivacyNamespace,
    input: &SpendInput,
    output_note: EvmNote,
    burn_recipient: payy_evm_client_interface::Address,
    user_key: [B256; 4],
) -> Element {
    tx_commitment(
        Element::from(client.inner.network.chain_id),
        address_to_element(client.inner.network.privacy_bridge),
        input.commitment,
        Element::ZERO,
        output_note.commitment(),
        Element::ZERO,
        address_to_field(burn_recipient),
        Element::ZERO,
        user_encrypted_key_hash(&user_key),
        Element::ZERO,
        Element::ZERO,
    )
}

pub(super) fn transfer_send_tx_commitment(
    client: &PrivacyNamespace,
    input: &SpendInput,
    output_note_self: EvmNote,
    output_note_recv: EvmNote,
    user_key: [B256; 4],
    recipient_key: [B256; 4],
) -> Element {
    tx_commitment(
        Element::from(client.inner.network.chain_id),
        address_to_element(client.inner.network.privacy_bridge),
        input.commitment,
        Element::ZERO,
        output_note_self.commitment(),
        output_note_recv.commitment(),
        Element::ZERO,
        Element::ZERO,
        user_encrypted_key_hash(&user_key),
        encrypted_key_hash(&recipient_key),
        receive_prefix(output_note_recv.owner),
    )
}

pub(super) fn transfer_claim_tx_commitment(
    client: &PrivacyNamespace,
    own: &OwnedSpendInput,
    incoming: &SpendInput,
    output_note: EvmNote,
    user_key: [B256; 4],
) -> Element {
    tx_commitment(
        Element::from(client.inner.network.chain_id),
        address_to_element(client.inner.network.privacy_bridge),
        own.commitment(),
        incoming.commitment,
        output_note.commitment(),
        Element::ZERO,
        Element::ZERO,
        Element::ZERO,
        user_encrypted_key_hash(&user_key),
        Element::ZERO,
        Element::ZERO,
    )
}
