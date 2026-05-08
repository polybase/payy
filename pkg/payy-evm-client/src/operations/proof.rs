#![allow(clippy::large_enum_variant)]

use element::Element;
use payy_evm_client_interface::{
    B256, PayyEvmTransactionRequest, PreparedOperationResult, PreparedPrivacyCall, PrivacyAccount,
    PrivacyOperationKind, PrivacyStatePreview,
};
use zk_circuits::circuits::generated;
use zk_primitives::EvmNote;

use super::prepared::Prepared;
use super::types::OperationBuilder;

pub(super) use super::proof_circuits::{
    prove_burn, prove_mint, prove_transfer_claim, prove_transfer_send,
};
pub(super) use super::proof_crypto::{
    chain_public_key, encrypt_chain_key, encrypt_key_for_public_key, encrypted_note,
    encryption_for_address,
};
pub(super) use super::proof_tx::{
    burn_tx_commitment, ensure_balance_at_least, ensure_u240, mint_tx_commitment,
    transfer_claim_tx_commitment, transfer_send_tx_commitment,
};

pub(super) type CircuitMerklePath = generated::submodules::evm_common_common::Merklepath;
pub(super) type CircuitOwnerSignature = generated::submodules::evm_common_signature::Ownersignature;

pub(super) struct ProvenCall {
    pub(super) verification_key_hash: B256,
    pub(super) proof: Vec<u8>,
    pub(super) public_inputs: Vec<B256>,
    pub(super) tx_commitment: B256,
    pub(super) recent_root: B256,
    pub(super) input_commitments: Vec<B256>,
    pub(super) input_nullifiers: Vec<B256>,
    pub(super) output_commitments: Vec<B256>,
    pub(super) user_encrypted_key: [B256; 4],
    pub(super) recipient_encrypted_key: [B256; 4],
}

pub(super) struct SpendInput {
    pub(super) note: EvmNote,
    pub(super) merkle_path: CircuitMerklePath,
    pub(super) recent_root: Element,
    pub(super) commitment: Element,
    pub(super) nullifier: Element,
}

pub(super) enum OwnedSpendInput {
    Real(Box<SpendInput>),
    Padding {
        merkle_path: CircuitMerklePath,
        recent_root: Element,
    },
}

impl OwnedSpendInput {
    pub(super) fn note(&self) -> EvmNote {
        match self {
            Self::Real(input) => input.note,
            Self::Padding { .. } => EvmNote::padding_note(),
        }
    }

    pub(super) fn merkle_path(&self) -> CircuitMerklePath {
        match self {
            Self::Real(input) => input.merkle_path.clone(),
            Self::Padding { merkle_path, .. } => merkle_path.clone(),
        }
    }

    pub(super) fn recent_root(&self) -> Element {
        match self {
            Self::Real(input) => input.recent_root,
            Self::Padding { recent_root, .. } => *recent_root,
        }
    }

    pub(super) fn commitment(&self) -> Element {
        match self {
            Self::Real(input) => input.commitment,
            Self::Padding { .. } => Element::ZERO,
        }
    }

    pub(super) fn nullifier(&self) -> Element {
        match self {
            Self::Real(input) => input.nullifier,
            Self::Padding { .. } => Element::ZERO,
        }
    }
}

pub(super) struct ClaimInputs {
    pub(super) own: OwnedSpendInput,
    pub(super) incoming: SpendInput,
}

impl<TPayload> OperationBuilder<TPayload> {
    pub(super) fn finish_prepared(
        &self,
        operation: PrivacyOperationKind,
        privacy_account: &PrivacyAccount,
        token: payy_evm_client_interface::Address,
        bridge_request: PayyEvmTransactionRequest,
        call: ProvenCall,
        payload: TPayload,
    ) -> Prepared<TPayload> {
        Prepared {
            result: PreparedOperationResult {
                prepared_call: PreparedPrivacyCall {
                    operation,
                    chain_id: self.client.inner.network.chain_id,
                    bridge_request,
                    verification_key_hash: call.verification_key_hash,
                    proof: call.proof,
                    public_inputs: call.public_inputs,
                    tx_commitment: call.tx_commitment,
                    state_preview: PrivacyStatePreview {
                        privacy_account: privacy_account.privacy_address(),
                        token,
                        recent_root: call.recent_root,
                        input_commitments: call.input_commitments,
                        input_nullifiers: call.input_nullifiers,
                        output_commitments: call.output_commitments,
                    },
                },
                payload,
            },
            client: self.client.clone(),
            submitter: None,
        }
    }
}
