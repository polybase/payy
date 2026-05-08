#![allow(clippy::large_futures)]

use payy_evm_client_interface::{
    Address, EvmAccount, PayyEvmTransactionRequest, PrivacyAccount, PrivacyOperationKind, Result,
    ValidationErrorKind,
};
use zk_primitives::EvmNote;

use super::params::{BurnParams, MintParams};
use super::prepared::Prepared;
use super::proof::{
    OwnedSpendInput, burn_tx_commitment, encryption_for_address, ensure_balance_at_least,
    ensure_u240, mint_tx_commitment, prove_burn, prove_mint,
};
use super::types::{OperationBuilder, OperationParams};
use crate::bridge::BridgeClient;
use crate::state::OwnedNoteGetParams;
use crate::util::{
    address_to_element, ensure_amount_non_zero, non_zero_address, random_nonzero_field,
};

impl OperationBuilder<()> {
    /// Prepare mint or burn.
    pub async fn prepare(self) -> Result<Prepared<()>> {
        match self.params.clone() {
            OperationParams::Mint(params) => self.prepare_mint(params).await,
            OperationParams::Burn(params) => self.prepare_burn(params).await,
            _ => unreachable!("payload type mismatch"),
        }
    }

    async fn prepare_mint(self, params: MintParams) -> Result<Prepared<()>> {
        ensure_amount_non_zero(params.amount)?;
        ensure_u240(params.amount)?;
        let evm_address = params.evm_account.address();
        let input = self
            .resolve_owned_spend_input(params.privacy_account.clone(), params.token, true)
            .await?;
        let input_note = input.note();
        let output_note = EvmNote {
            kind: element::Element::ONE,
            token: address_to_element(params.token),
            nonce: if input_note.kind.is_zero() {
                element::Element::ZERO
            } else {
                input_note.nonce + element::Element::ONE
            },
            psi: random_nonzero_field(),
            owner: params.privacy_account.privacy_address().owner()?,
            value: input_note.value + params.amount,
        };
        ensure_u240(output_note.value)?;
        let (symmetric_key, encrypted_note, chain_key, user_key) =
            encryption_for_address(&self.client, output_note, &params.privacy_account).await?;
        let signature = self.sign_owner(
            params.privacy_account.clone(),
            mint_tx_commitment(&self.client, &input, output_note, evm_address, user_key),
        )?;
        let call = prove_mint(
            &self.client,
            signature,
            input,
            output_note,
            evm_address,
            symmetric_key,
            user_key,
            encrypted_note,
            chain_key,
        )
        .await?;
        let request = PayyEvmTransactionRequest {
            from: Some(evm_address),
            to: self.client.inner.network.privacy_bridge,
            data: BridgeClient::encode_mint_call(
                call.verification_key_hash,
                &call.proof,
                &call.public_inputs,
                call.user_encrypted_key,
            ),
            value: 0,
            gas_limit: None,
        };
        let mut prepared = self.finish_prepared(
            PrivacyOperationKind::Mint,
            &params.privacy_account,
            params.token,
            request,
            call,
            (),
        );
        if let EvmAccount::Signer(account) = params.evm_account {
            prepared.submitter = Some(account.submitter);
        }
        Ok(prepared)
    }

    async fn prepare_burn(self, params: BurnParams) -> Result<Prepared<()>> {
        ensure_amount_non_zero(params.amount)?;
        ensure_u240(params.amount)?;
        if !non_zero_address(params.evm_recipient) {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::EvmRecipientZero,
            });
        }
        let input = self
            .resolve_owned_spend_input(params.privacy_account.clone(), params.token, false)
            .await?;
        let OwnedSpendInput::Real(input) = input else {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::MissingOwnedNote,
            });
        };
        ensure_balance_at_least(input.note, params.amount)?;
        let output_note = EvmNote {
            kind: element::Element::ONE,
            token: input.note.token,
            nonce: input.note.nonce + element::Element::ONE,
            psi: random_nonzero_field(),
            owner: input.note.owner,
            value: input.note.value - params.amount,
        };
        let (symmetric_key, encrypted_note, chain_key, user_key) =
            encryption_for_address(&self.client, output_note, &params.privacy_account).await?;
        let signature = self.sign_owner(
            params.privacy_account.clone(),
            burn_tx_commitment(
                &self.client,
                &input,
                output_note,
                params.evm_recipient,
                user_key,
            ),
        )?;
        let call = prove_burn(
            &self.client,
            signature,
            *input,
            output_note,
            params.evm_recipient,
            params.amount,
            symmetric_key,
            user_key,
            encrypted_note,
            chain_key,
        )
        .await?;
        let request = PayyEvmTransactionRequest {
            from: None,
            to: self.client.inner.network.privacy_bridge,
            data: BridgeClient::encode_burn_call(
                call.verification_key_hash,
                &call.proof,
                &call.public_inputs,
                call.user_encrypted_key,
            ),
            value: 0,
            gas_limit: None,
        };
        Ok(self.finish_prepared(
            PrivacyOperationKind::Burn,
            &params.privacy_account,
            params.token,
            request,
            call,
            (),
        ))
    }
}

impl<TPayload> OperationBuilder<TPayload> {
    pub(super) async fn resolve_owned_note(
        &self,
        privacy_account: PrivacyAccount,
        token: Address,
    ) -> Result<payy_evm_client_interface::OwnedNoteState> {
        self.client
            .resolve_owned_note(
                OwnedNoteGetParams {
                    privacy_account,
                    token,
                },
                self.checkpoint.clone(),
            )
            .await
    }
}
