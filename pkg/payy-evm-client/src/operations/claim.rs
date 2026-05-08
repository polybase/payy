// lint-long-file-override allow-max-lines=300
#![allow(clippy::large_futures)]

use element::Element;
use payy_evm_client_interface::{
    ClaimLink, IncomingNote, ParsedClaimLink, PrivacyAccount, PrivacyOperationKind, Result,
    ValidationErrorKind,
};
use zk_primitives::EvmNote;
use zk_primitives::field_to_address;

use super::prepared::Prepared;
use super::proof::{
    encryption_for_address, ensure_u240, prove_transfer_claim, transfer_claim_tx_commitment,
};
use super::types::{OperationBuilder, OperationParams};
use super::validate::{
    IncomingTransferSource, validate_claim_publication, validate_incoming_note,
    validate_incoming_transfer,
};
use crate::bridge::BridgeClient;
use crate::util::random_nonzero_field;

impl OperationBuilder<IncomingNote> {
    /// Prepare discovered-note claim.
    pub async fn prepare(self) -> Result<Prepared<IncomingNote>> {
        match self.params.clone() {
            OperationParams::ClaimNote {
                incoming_note,
                account,
            } => self.prepare_claim_note(incoming_note, account).await,
            _ => unreachable!("payload type mismatch"),
        }
    }

    async fn prepare_claim_note(
        self,
        incoming_note: IncomingNote,
        account: Option<PrivacyAccount>,
    ) -> Result<Prepared<IncomingNote>> {
        validate_incoming_note(&incoming_note)?;
        validate_claim_publication(&self.client, incoming_note.note).await?;
        let account = self.resolve_direct_claim_account(incoming_note.note.owner, account)?;
        self.prepare_direct_claim(incoming_note.note, account, incoming_note)
            .await
    }
}

impl OperationBuilder<ParsedClaimLink> {
    /// Prepare link claim.
    pub async fn prepare(self) -> Result<Prepared<ParsedClaimLink>> {
        match self.params.clone() {
            OperationParams::ClaimLink { link, account } => {
                self.prepare_claim_link(&link, account).await
            }
            _ => unreachable!("payload type mismatch"),
        }
    }

    async fn prepare_claim_link(
        self,
        link: &ClaimLink,
        account: Option<PrivacyAccount>,
    ) -> Result<Prepared<ParsedClaimLink>> {
        let parsed = self.client.links().parse(&link.value)?;
        if parsed.incoming_transfer.is_some() && account.is_none() {
            return Err(payy_evm_client_interface::Error::MissingCapability {
                capability: "claim_account",
            });
        }
        let selected = if let Some(incoming_transfer) = &parsed.incoming_transfer {
            validate_incoming_transfer(incoming_transfer, IncomingTransferSource::Link)?;
            validate_claim_publication(&self.client, incoming_transfer.note).await?;
            account.ok_or(payy_evm_client_interface::Error::MissingCapability {
                capability: "claim_account",
            })?
        } else {
            let note = parsed
                .direct_note
                .as_ref()
                .map_or(EvmNote::padding_note(), |direct| direct.note);
            validate_claim_publication(&self.client, note).await?;
            self.resolve_direct_claim_account(note.owner, account)?
        };
        if let Some(incoming_transfer) = &parsed.incoming_transfer {
            self.prepare_ephemeral_link_claim(incoming_transfer.clone(), selected, parsed)
                .await
        } else {
            let note = parsed
                .direct_note
                .as_ref()
                .map_or(EvmNote::padding_note(), |direct| direct.note);
            self.prepare_direct_claim(note, selected, parsed).await
        }
    }
}

impl<TPayload> OperationBuilder<TPayload> {
    async fn prepare_direct_claim(
        self,
        incoming_note: EvmNote,
        account: PrivacyAccount,
        payload: TPayload,
    ) -> Result<Prepared<TPayload>> {
        let inputs = self.claim_inputs(account.clone(), incoming_note).await?;
        let own_note = inputs.own.note();
        let output_note = EvmNote {
            kind: Element::ONE,
            token: incoming_note.token,
            nonce: if own_note.kind.is_zero() {
                Element::ZERO
            } else {
                own_note.nonce + Element::ONE
            },
            psi: random_nonzero_field(),
            owner: account.privacy_address().owner()?,
            value: own_note.value + incoming_note.value,
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
        let incoming_note_signature = self.sign_owner(account.clone(), signed)?;
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
        let request = payy_evm_client_interface::PayyEvmTransactionRequest {
            from: None,
            to: self.client.inner.network.privacy_bridge,
            data: BridgeClient::encode_transfer_call(
                call.verification_key_hash,
                &call.proof,
                &call.public_inputs,
                call.user_encrypted_key,
                [[0; 32]; 4],
                [0; 32],
            ),
            value: 0,
            gas_limit: None,
        };
        Ok(self.finish_prepared(
            PrivacyOperationKind::TransferClaim,
            &account,
            field_to_address(incoming_note.token),
            request,
            call,
            payload,
        ))
    }

    async fn prepare_ephemeral_link_claim(
        self,
        incoming_transfer: payy_evm_client_interface::IncomingTransfer,
        account: PrivacyAccount,
        payload: TPayload,
    ) -> Result<Prepared<TPayload>> {
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
        let request = payy_evm_client_interface::PayyEvmTransactionRequest {
            from: None,
            to: self.client.inner.network.privacy_bridge,
            data: BridgeClient::encode_transfer_call(
                call.verification_key_hash,
                &call.proof,
                &call.public_inputs,
                call.user_encrypted_key,
                [[0; 32]; 4],
                [0; 32],
            ),
            value: 0,
            gas_limit: None,
        };
        Ok(self.finish_prepared(
            PrivacyOperationKind::TransferClaim,
            &account,
            field_to_address(incoming_transfer.note.token),
            request,
            call,
            payload,
        ))
    }

    fn resolve_direct_claim_account(
        &self,
        owner: element::Element,
        account: Option<PrivacyAccount>,
    ) -> Result<PrivacyAccount> {
        if let Some(account) = account {
            if account.privacy_address().owner()? != owner {
                return Err(payy_evm_client_interface::Error::Validation {
                    kind: ValidationErrorKind::PrivacyAccountMismatch,
                });
            }
            return Ok(account);
        }
        for candidate in self.client.inner.privacy_signer()?.accounts()? {
            if candidate.privacy_address().owner()? == owner {
                return Ok(candidate);
            }
        }
        Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::PrivacyAccountMismatch,
        })
    }
}
