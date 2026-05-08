// lint-long-file-override allow-max-lines=300
#![allow(clippy::large_futures, clippy::too_many_lines)]

use element::Element;
use payy_evm_client_interface::{
    ClaimLink, IncomingTransfer, PrivacyAccount, PrivacyOperationKind, Result,
};
use zk_primitives::{EvmNote, field_to_address};

use super::params::EphemeralSendParams;
use super::prepared::Prepared;
use super::proof::{
    OwnedSpendInput, chain_public_key, encrypt_chain_key, encrypt_key_for_public_key,
    encrypted_note, encryption_for_address, ensure_balance_at_least, ensure_u240,
    prove_transfer_claim, prove_transfer_send, transfer_claim_tx_commitment,
    transfer_send_tx_commitment,
};
use super::send::transfer_request;
use super::types::{OperationBuilder, OperationParams};
use super::validate::{
    IncomingTransferSource, validate_claim_publication, validate_incoming_transfer,
};
use crate::bridge::BridgeClient;
use crate::links::{encode_ephemeral_claim_link, ephemeral_owner};
use crate::util::{
    address_to_element, element_to_b256, ensure_amount_non_zero, random_nonzero_field,
    random_u240_field,
};

impl OperationBuilder<IncomingTransfer> {
    /// Prepare ephemeral send or claim.
    pub async fn prepare(self) -> Result<Prepared<IncomingTransfer>> {
        match self.params.clone() {
            OperationParams::EphemeralSend(params) => self.prepare_ephemeral_send(&params).await,
            OperationParams::ClaimEphemeral {
                incoming_transfer,
                account,
            } => {
                self.prepare_claim_ephemeral(incoming_transfer, account)
                    .await
            }
            _ => unreachable!("payload type mismatch"),
        }
    }

    /// Prepare ephemeral send and attach a claim link.
    pub async fn link(
        self,
        message: Option<&str>,
    ) -> Result<Prepared<(IncomingTransfer, ClaimLink)>> {
        let prepared = self.prepare().await?;
        let link = encode_ephemeral_claim_link(prepared.payload(), message);
        Ok(prepared.map_payload(|payload| (payload, link)))
    }

    async fn prepare_ephemeral_send(
        self,
        params: &EphemeralSendParams,
    ) -> Result<Prepared<IncomingTransfer>> {
        ensure_amount_non_zero(params.amount)?;
        ensure_u240(params.amount)?;
        let input = self
            .resolve_owned_spend_input(params.privacy_account.clone(), params.token, false)
            .await?;
        let OwnedSpendInput::Real(input) = input else {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: payy_evm_client_interface::ValidationErrorKind::MissingOwnedNote,
            });
        };
        ensure_balance_at_least(input.note, params.amount)?;
        let key = self
            .signer_for_account(&params.privacy_account)?
            .generate_ephemeral_key()?;
        let output_note_self = EvmNote {
            kind: Element::ONE,
            token: input.note.token,
            nonce: input.note.nonce + Element::ONE,
            psi: random_nonzero_field(),
            owner: input.note.owner,
            value: input.note.value - params.amount,
        };
        let note = EvmNote {
            kind: Element::ONE,
            token: address_to_element(params.token),
            nonce: Element::ZERO,
            psi: random_nonzero_field(),
            owner: ephemeral_owner(&key.private_key)?,
            value: params.amount,
        };
        let commitment = element_to_b256(note.commitment());
        let chain_key = chain_public_key(&self.client).await?;
        let sender_symmetric_key = random_u240_field();
        let recipient_symmetric_key = random_u240_field();
        let sender_encrypted_note = encrypted_note(output_note_self, sender_symmetric_key);
        let recipient_encrypted_note = encrypted_note(note, recipient_symmetric_key);
        let sender_chain_key = encrypt_chain_key(sender_symmetric_key, chain_key[0], chain_key[1])?;
        let recipient_chain_key =
            encrypt_chain_key(recipient_symmetric_key, chain_key[0], chain_key[1])?;
        let (sender_public_x, sender_public_y) =
            params.privacy_account.privacy_address().public_key()?;
        let user_key =
            encrypt_key_for_public_key(sender_symmetric_key, sender_public_x, sender_public_y)?;
        let (recipient_public_x, recipient_public_y) = key.privacy_address.public_key()?;
        let recipient_key = encrypt_key_for_public_key(
            recipient_symmetric_key,
            recipient_public_x,
            recipient_public_y,
        )?;
        let signature = self.sign_owner(
            params.privacy_account.clone(),
            transfer_send_tx_commitment(
                &self.client,
                &input,
                output_note_self,
                note,
                user_key,
                recipient_key,
            ),
        )?;
        let call = prove_transfer_send(
            &self.client,
            signature,
            *input,
            output_note_self,
            note,
            sender_symmetric_key,
            recipient_symmetric_key,
            user_key,
            recipient_key,
            sender_encrypted_note,
            recipient_encrypted_note,
            sender_chain_key,
            recipient_chain_key,
        )
        .await?;
        let request = transfer_request(
            self.client.inner.network.privacy_bridge,
            BridgeClient::encode_transfer_call(
                call.verification_key_hash,
                &call.proof,
                &call.public_inputs,
                call.user_encrypted_key,
                call.recipient_encrypted_key,
                params.bridge_memo.unwrap_or([0; 32]),
            ),
        );
        let source_bridge_tx_hash = self.client.bridge().compute_tx_hash(
            call.verification_key_hash,
            &call.proof,
            &call.public_inputs,
        );
        Ok(self.finish_prepared(
            PrivacyOperationKind::TransferSend,
            &params.privacy_account,
            params.token,
            request,
            call,
            IncomingTransfer {
                note,
                commitment,
                ephemeral_private_key: key.private_key,
                source_tx_hash: None,
                source_bridge_tx_hash: Some(source_bridge_tx_hash),
            },
        ))
    }

    async fn prepare_claim_ephemeral(
        self,
        incoming_transfer: IncomingTransfer,
        account: Option<PrivacyAccount>,
    ) -> Result<Prepared<IncomingTransfer>> {
        let account = account.ok_or(payy_evm_client_interface::Error::MissingCapability {
            capability: "claim_account",
        })?;
        validate_incoming_transfer(&incoming_transfer, IncomingTransferSource::Direct)?;
        validate_claim_publication(&self.client, incoming_transfer.note).await?;
        self.prepare_claim_ephemeral_proven(incoming_transfer, account)
            .await
    }

    async fn prepare_claim_ephemeral_proven(
        self,
        incoming_transfer: IncomingTransfer,
        account: PrivacyAccount,
    ) -> Result<Prepared<IncomingTransfer>> {
        let inputs = self
            .claim_inputs(account.clone(), incoming_transfer.note)
            .await?;
        let own_note = inputs.own.note();
        let output_note = EvmNote {
            kind: Element::ONE,
            token: incoming_transfer.note.token,
            nonce: if own_note.kind.is_zero() {
                Element::ZERO
            } else {
                own_note.nonce + Element::ONE
            },
            psi: random_nonzero_field(),
            owner: account.privacy_address().owner()?,
            value: own_note.value + incoming_transfer.note.value,
        };
        ensure_u240(output_note.value)?;
        let (symmetric_key, encrypted_note, chain_key, user_key) =
            encryption_for_address(&self.client, output_note, &account).await?;
        let signed = transfer_claim_tx_commitment(
            &self.client,
            &inputs.own,
            &inputs.incoming,
            output_note,
            user_key,
        );
        let recipient_signature = self.sign_owner(account.clone(), signed)?;
        let incoming_note_signature =
            self.sign_ephemeral_owner(incoming_transfer.ephemeral_private_key, signed)?;
        let call = prove_transfer_claim(
            &self.client,
            recipient_signature,
            incoming_note_signature,
            inputs,
            output_note,
            symmetric_key,
            user_key,
            encrypted_note,
            chain_key,
        )
        .await?;
        let request = transfer_request(
            self.client.inner.network.privacy_bridge,
            BridgeClient::encode_transfer_call(
                call.verification_key_hash,
                &call.proof,
                &call.public_inputs,
                call.user_encrypted_key,
                [[0; 32]; 4],
                [0; 32],
            ),
        );
        Ok(self.finish_prepared(
            PrivacyOperationKind::TransferClaim,
            &account,
            field_to_address(incoming_transfer.note.token),
            request,
            call,
            incoming_transfer,
        ))
    }
}
