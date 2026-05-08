#![allow(clippy::large_futures, clippy::too_many_lines)]

use element::Element;
use payy_evm_client_interface::{
    ClaimLink, DirectSendDelivery, PayyEvmTransactionRequest, PrivacyOperationKind, Result,
};
use zk_primitives::EvmNote;

use super::params::DirectSendParams;
use super::prepared::Prepared;
use super::proof::{
    OwnedSpendInput, chain_public_key, encrypt_chain_key, encrypt_key_for_public_key,
    encrypted_note, ensure_balance_at_least, ensure_u240, prove_transfer_send,
    transfer_send_tx_commitment,
};
use super::types::{OperationBuilder, OperationParams};
use crate::bridge::BridgeClient;
use crate::links::encode_direct_claim_link;
use crate::util::{
    address_to_element, element_to_b256, ensure_amount_non_zero, random_nonzero_field,
    random_u240_field,
};

impl OperationBuilder<DirectSendDelivery> {
    /// Prepare direct send.
    #[allow(clippy::unused_async)]
    pub async fn prepare(self) -> Result<Prepared<DirectSendDelivery>> {
        match self.params.clone() {
            OperationParams::DirectSend(params) => self.prepare_direct_send(&params).await,
            _ => unreachable!("payload type mismatch"),
        }
    }

    /// Prepare direct send and attach a claim link.
    pub async fn link(
        self,
        message: Option<&str>,
    ) -> Result<Prepared<(DirectSendDelivery, ClaimLink)>> {
        let prepared = self.prepare().await?;
        let link = encode_direct_claim_link(prepared.payload().note, message);
        Ok(prepared.map_payload(|payload| (payload, link)))
    }

    async fn prepare_direct_send(
        self,
        params: &DirectSendParams,
    ) -> Result<Prepared<DirectSendDelivery>> {
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
        let owner = params.recipient.owner()?;
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
            owner,
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
        let (recipient_public_x, recipient_public_y) = params.recipient.public_key()?;
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
            DirectSendDelivery {
                recipient: params.recipient,
                note,
                commitment,
                source_tx_hash: None,
                source_bridge_tx_hash: Some(source_bridge_tx_hash),
            },
        ))
    }
}

pub(super) fn transfer_request(to: [u8; 20], data: Vec<u8>) -> PayyEvmTransactionRequest {
    PayyEvmTransactionRequest {
        from: None,
        to,
        data,
        value: 0,
        gas_limit: None,
    }
}
