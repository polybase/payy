// lint-long-file-override allow-max-lines=300
#![allow(clippy::large_futures, clippy::too_many_arguments)]

use element::Element;
use payy_evm_client_interface::{B256, Result, ValidationErrorKind};
use payy_evm_client_prover_interface::PrivacyCircuit;
use zk_circuits::circuits::generated;
use zk_primitives::{
    EvmNote, address_to_field, encrypted_key_hash, receive_prefix, user_encrypted_key_hash,
};

use super::proof::{CircuitOwnerSignature, ClaimInputs, OwnedSpendInput, ProvenCall, SpendInput};
use super::proof_crypto::chain_public_key;
use super::proof_tx::{
    burn_tx_commitment, mint_tx_commitment, output_nonce_hash, transfer_claim_tx_commitment,
    transfer_send_tx_commitment,
};
use super::proof_witness::prove_witness;
use crate::client::PrivacyNamespace;
use crate::util::address_to_element;

type CircuitNote = generated::submodules::evm_common_note::Evmnote;

pub(super) async fn prove_mint(
    client: &PrivacyNamespace,
    owner_signature: CircuitOwnerSignature,
    input: OwnedSpendInput,
    output_note: EvmNote,
    mint_from: payy_evm_client_interface::Address,
    symmetric_key: Element,
    user_key: [B256; 4],
    encrypted_note: [Element; 5],
    chain_key: [Element; 3],
) -> Result<ProvenCall> {
    let input_note = input.note();
    let output_commitment = output_note.commitment();
    let input_commitment = input.commitment();
    let user_hash = user_encrypted_key_hash(&user_key);
    let signed = mint_tx_commitment(client, &input, output_note, mint_from, user_key);
    let witness = generated::mint::MintInput {
        input_note: circuit_note(input_note),
        input_merkle_path: input.merkle_path(),
        output_note: circuit_note(output_note),
        owner_signature,
        symmetric_key,
        chain_id: Element::from(client.inner.network.chain_id),
        bridge_address: address_to_element(client.inner.network.privacy_bridge),
        recent_root: input.recent_root(),
        input_nullifiers: [input.nullifier(), Element::ZERO],
        output_commitments: [output_commitment, Element::ZERO],
        nonce_hash: output_nonce_hash(input_note, output_note),
        user_encrypted_key_hash: user_hash,
        recipient_encrypted_key_hash: Element::ZERO,
        sender_encrypted_note: encrypted_note,
        recipient_encrypted_note: [Element::ZERO; 5],
        sender_chain_encrypted_key: chain_key,
        recipient_chain_encrypted_key: [Element::ZERO; 3],
        chain_public_key: chain_public_key(client).await?,
        token: output_note.token,
        burn_recipient: Element::ZERO,
        value: output_note.value - input_note.value,
        mint_from: address_to_field(mint_from),
        receive_prefix: Element::ZERO,
    };
    prove_witness(
        client,
        PrivacyCircuit::Mint,
        signed,
        input.recent_root(),
        vec![input_commitment],
        vec![input.nullifier()],
        vec![output_commitment],
        user_key,
        [[0; 32]; 4],
        witness,
    )
    .await
}

pub(super) async fn prove_burn(
    client: &PrivacyNamespace,
    owner_signature: CircuitOwnerSignature,
    input: SpendInput,
    output_note: EvmNote,
    burn_recipient: payy_evm_client_interface::Address,
    burn_value: Element,
    symmetric_key: Element,
    user_key: [B256; 4],
    encrypted_note: [Element; 5],
    chain_key: [Element; 3],
) -> Result<ProvenCall> {
    let output_commitment = output_note.commitment();
    let user_hash = user_encrypted_key_hash(&user_key);
    let signed = burn_tx_commitment(client, &input, output_note, burn_recipient, user_key);
    let witness = generated::burn::BurnInput {
        input_note: circuit_note(input.note),
        input_merkle_path: input.merkle_path,
        output_note: circuit_note(output_note),
        owner_signature,
        burn_recipient_private: address_to_field(burn_recipient),
        symmetric_key,
        chain_id: Element::from(client.inner.network.chain_id),
        bridge_address: address_to_element(client.inner.network.privacy_bridge),
        recent_root: input.recent_root,
        input_nullifiers: [input.nullifier, Element::ZERO],
        output_commitments: [output_commitment, Element::ZERO],
        nonce_hash: output_nonce_hash(input.note, output_note),
        user_encrypted_key_hash: user_hash,
        recipient_encrypted_key_hash: Element::ZERO,
        sender_encrypted_note: encrypted_note,
        recipient_encrypted_note: [Element::ZERO; 5],
        sender_chain_encrypted_key: chain_key,
        recipient_chain_encrypted_key: [Element::ZERO; 3],
        chain_public_key: chain_public_key(client).await?,
        token: input.note.token,
        burn_recipient: address_to_field(burn_recipient),
        value: burn_value,
        mint_from: Element::ZERO,
        receive_prefix: Element::ZERO,
    };
    prove_witness(
        client,
        PrivacyCircuit::Burn,
        signed,
        input.recent_root,
        vec![input.commitment],
        vec![input.nullifier],
        vec![output_commitment],
        user_key,
        [[0; 32]; 4],
        witness,
    )
    .await
}

pub(super) async fn prove_transfer_send(
    client: &PrivacyNamespace,
    owner_signature: CircuitOwnerSignature,
    input: SpendInput,
    output_note_self: EvmNote,
    output_note_recv: EvmNote,
    sender_symmetric_key: Element,
    recipient_symmetric_key: Element,
    user_key: [B256; 4],
    recipient_key: [B256; 4],
    sender_encrypted_note: [Element; 5],
    recipient_encrypted_note: [Element; 5],
    sender_chain_key: [Element; 3],
    recipient_chain_key: [Element; 3],
) -> Result<ProvenCall> {
    let signed = transfer_send_tx_commitment(
        client,
        &input,
        output_note_self,
        output_note_recv,
        user_key,
        recipient_key,
    );
    let output_commitment_0 = output_note_self.commitment();
    let output_commitment_1 = output_note_recv.commitment();
    let witness = generated::transfer_send::TransferSendInput {
        input_note: circuit_note(input.note),
        input_merkle_path: input.merkle_path,
        output_note_self: circuit_note(output_note_self),
        output_note_recv: circuit_note(output_note_recv),
        owner_signature,
        sender_symmetric_key,
        recipient_symmetric_key,
        chain_id: Element::from(client.inner.network.chain_id),
        bridge_address: address_to_element(client.inner.network.privacy_bridge),
        recent_root: input.recent_root,
        input_nullifiers: [input.nullifier, Element::ZERO],
        output_commitments: [output_commitment_0, output_commitment_1],
        nonce_hash: output_nonce_hash(input.note, output_note_self),
        user_encrypted_key_hash: user_encrypted_key_hash(&user_key),
        recipient_encrypted_key_hash: encrypted_key_hash(&recipient_key),
        sender_encrypted_note,
        recipient_encrypted_note,
        sender_chain_encrypted_key: sender_chain_key,
        recipient_chain_encrypted_key: recipient_chain_key,
        chain_public_key: chain_public_key(client).await?,
        token: Element::ZERO,
        burn_recipient: Element::ZERO,
        value: Element::ZERO,
        mint_from: Element::ZERO,
        receive_prefix: receive_prefix(output_note_recv.owner),
    };
    prove_witness(
        client,
        PrivacyCircuit::TransferSend,
        signed,
        input.recent_root,
        vec![input.commitment],
        vec![input.nullifier],
        vec![output_commitment_0, output_commitment_1],
        user_key,
        recipient_key,
        witness,
    )
    .await
}

pub(super) async fn prove_transfer_claim(
    client: &PrivacyNamespace,
    recipient_signature: CircuitOwnerSignature,
    incoming_note_signature: CircuitOwnerSignature,
    inputs: ClaimInputs,
    output_note: EvmNote,
    sender_symmetric_key: Element,
    user_key: [B256; 4],
    sender_encrypted_note: [Element; 5],
    sender_chain_key: [Element; 3],
) -> Result<ProvenCall> {
    let recent_root = inputs.incoming.recent_root;
    if let OwnedSpendInput::Real(own) = &inputs.own
        && own.recent_root != recent_root
    {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::CommitmentMismatch,
        });
    }
    let signed =
        transfer_claim_tx_commitment(client, &inputs.own, &inputs.incoming, output_note, user_key);
    let output_commitment = output_note.commitment();
    let input_note_own = inputs.own.note();
    let witness = generated::transfer_claim::TransferClaimInput {
        input_note_own: circuit_note(input_note_own),
        input_note_incoming: circuit_note(inputs.incoming.note),
        input_merkle_path_own: inputs.own.merkle_path(),
        input_merkle_path_incoming: inputs.incoming.merkle_path,
        output_note: circuit_note(output_note),
        recipient_signature,
        incoming_note_signature,
        sender_symmetric_key,
        chain_id: Element::from(client.inner.network.chain_id),
        bridge_address: address_to_element(client.inner.network.privacy_bridge),
        recent_root,
        input_nullifiers: [inputs.own.nullifier(), inputs.incoming.nullifier],
        output_commitments: [output_commitment, Element::ZERO],
        nonce_hash: output_nonce_hash(input_note_own, output_note),
        user_encrypted_key_hash: user_encrypted_key_hash(&user_key),
        recipient_encrypted_key_hash: Element::ZERO,
        sender_encrypted_note,
        recipient_encrypted_note: [Element::ZERO; 5],
        sender_chain_encrypted_key: sender_chain_key,
        recipient_chain_encrypted_key: [Element::ZERO; 3],
        chain_public_key: chain_public_key(client).await?,
        token: Element::ZERO,
        burn_recipient: Element::ZERO,
        value: Element::ZERO,
        mint_from: Element::ZERO,
        receive_prefix: Element::ZERO,
    };
    let input_commitments = [inputs.own.commitment(), inputs.incoming.commitment]
        .into_iter()
        .filter(|commitment| !commitment.is_zero())
        .collect::<Vec<_>>();
    let input_nullifiers = [inputs.own.nullifier(), inputs.incoming.nullifier]
        .into_iter()
        .filter(|nullifier| !nullifier.is_zero())
        .collect::<Vec<_>>();
    prove_witness(
        client,
        PrivacyCircuit::TransferClaim,
        signed,
        recent_root,
        input_commitments,
        input_nullifiers,
        vec![output_commitment],
        user_key,
        [[0; 32]; 4],
        witness,
    )
    .await
}

fn circuit_note(note: EvmNote) -> CircuitNote {
    CircuitNote {
        kind: note.kind,
        token: note.token,
        nonce: note.nonce,
        psi: note.psi,
        owner: note.owner,
        value: note.value,
    }
}
