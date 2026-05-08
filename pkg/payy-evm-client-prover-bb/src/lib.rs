#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]
// lint-long-file-override allow-max-lines=300

//! Barretenberg-backed Payy-EVM client prover.

use std::sync::Arc;

use async_trait::async_trait;
use barretenberg_interface::BbBackend;
use contextful::ResultContextExt;
use element::Element;
use payy_evm_client_prover_interface::{
    PrivacyCircuit, PrivacyProver, ProveOutput, ProveRequest, Result,
};
use proof_decode::decode_proof;
use zk_circuits::circuits::generated;

mod proof_decode;

#[cfg(test)]
mod tests;

/// Barretenberg-backed prover.
#[derive(Clone)]
pub struct BarretenbergPrivacyProver {
    backend: Arc<dyn BbBackend>,
}

impl BarretenbergPrivacyProver {
    /// Build a prover from a Barretenberg backend.
    #[must_use]
    pub fn new(backend: Arc<dyn BbBackend>) -> Self {
        Self { backend }
    }

    /// Return the configured backend.
    #[must_use]
    pub fn backend(&self) -> Arc<dyn BbBackend> {
        self.backend.clone()
    }
}

#[async_trait]
impl PrivacyProver for BarretenbergPrivacyProver {
    async fn prove(&self, request: ProveRequest) -> Result<ProveOutput> {
        match request.circuit {
            PrivacyCircuit::Mint => self.prove_mint(&request.witness).await,
            PrivacyCircuit::Burn => self.prove_burn(&request.witness).await,
            PrivacyCircuit::TransferSend => self.prove_transfer_send(&request.witness).await,
            PrivacyCircuit::TransferClaim => self.prove_transfer_claim(&request.witness).await,
        }
    }
}

impl BarretenbergPrivacyProver {
    async fn prove_mint(&self, witness: &[u8]) -> Result<ProveOutput> {
        let proof = self
            .backend
            .prove(
                generated::mint::PROGRAM.as_bytes(),
                generated::mint::BYTECODE.as_ref(),
                generated::mint::KEY,
                witness,
                false,
            )
            .await
            .context("prove mint circuit")?;
        let proof = decode_proof::<generated::mint::MintPublicInputs>(PrivacyCircuit::Mint, proof)?;
        Ok(ProveOutput {
            verification_key_hash: verification_key_hash(*generated::mint::VERIFICATION_KEY_HASH),
            proof: proof.proof,
            public_inputs: mint_public_inputs_to_vec(&proof.public_inputs),
        })
    }

    async fn prove_burn(&self, witness: &[u8]) -> Result<ProveOutput> {
        let proof = self
            .backend
            .prove(
                generated::burn::PROGRAM.as_bytes(),
                generated::burn::BYTECODE.as_ref(),
                generated::burn::KEY,
                witness,
                false,
            )
            .await
            .context("prove burn circuit")?;
        let proof = decode_proof::<generated::burn::BurnPublicInputs>(PrivacyCircuit::Burn, proof)?;
        Ok(ProveOutput {
            verification_key_hash: verification_key_hash(*generated::burn::VERIFICATION_KEY_HASH),
            proof: proof.proof,
            public_inputs: burn_public_inputs_to_vec(&proof.public_inputs),
        })
    }

    async fn prove_transfer_send(&self, witness: &[u8]) -> Result<ProveOutput> {
        let proof = self
            .backend
            .prove(
                generated::transfer_send::PROGRAM.as_bytes(),
                generated::transfer_send::BYTECODE.as_ref(),
                generated::transfer_send::KEY,
                witness,
                false,
            )
            .await
            .context("prove transfer_send circuit")?;
        let proof = decode_proof::<generated::transfer_send::TransferSendPublicInputs>(
            PrivacyCircuit::TransferSend,
            proof,
        )?;
        Ok(ProveOutput {
            verification_key_hash: verification_key_hash(
                *generated::transfer_send::VERIFICATION_KEY_HASH,
            ),
            proof: proof.proof,
            public_inputs: transfer_send_public_inputs_to_vec(&proof.public_inputs),
        })
    }

    async fn prove_transfer_claim(&self, witness: &[u8]) -> Result<ProveOutput> {
        let proof = self
            .backend
            .prove(
                generated::transfer_claim::PROGRAM.as_bytes(),
                generated::transfer_claim::BYTECODE.as_ref(),
                generated::transfer_claim::KEY,
                witness,
                false,
            )
            .await
            .context("prove transfer_claim circuit")?;
        let proof = decode_proof::<generated::transfer_claim::TransferClaimPublicInputs>(
            PrivacyCircuit::TransferClaim,
            proof,
        )?;
        Ok(ProveOutput {
            verification_key_hash: verification_key_hash(
                *generated::transfer_claim::VERIFICATION_KEY_HASH,
            ),
            proof: proof.proof,
            public_inputs: transfer_claim_public_inputs_to_vec(&proof.public_inputs),
        })
    }
}

fn verification_key_hash(hash: element::Base) -> [u8; 32] {
    Element::from_base(hash).to_be_bytes()
}

macro_rules! public_inputs_to_vec {
    ($fn_name:ident, $ty:path) => {
        fn $fn_name(inputs: &$ty) -> Vec<[u8; 32]> {
            canonical_public_inputs_to_vec(&PublicInputParts {
                chain_id: inputs.chain_id,
                bridge_address: inputs.bridge_address,
                recent_root: inputs.recent_root,
                input_nullifiers: inputs.input_nullifiers,
                output_commitments: inputs.output_commitments,
                nonce_hash: inputs.nonce_hash,
                user_encrypted_key_hash: inputs.user_encrypted_key_hash,
                recipient_encrypted_key_hash: inputs.recipient_encrypted_key_hash,
                sender_encrypted_note: inputs.sender_encrypted_note,
                recipient_encrypted_note: inputs.recipient_encrypted_note,
                sender_chain_encrypted_key: inputs.sender_chain_encrypted_key,
                recipient_chain_encrypted_key: inputs.recipient_chain_encrypted_key,
                chain_public_key: inputs.chain_public_key,
                token: inputs.token,
                burn_recipient: inputs.burn_recipient,
                value: inputs.value,
                mint_from: inputs.mint_from,
                receive_prefix: inputs.receive_prefix,
            })
        }
    };
}

public_inputs_to_vec!(mint_public_inputs_to_vec, generated::mint::MintPublicInputs);
public_inputs_to_vec!(burn_public_inputs_to_vec, generated::burn::BurnPublicInputs);
public_inputs_to_vec!(
    transfer_send_public_inputs_to_vec,
    generated::transfer_send::TransferSendPublicInputs
);
public_inputs_to_vec!(
    transfer_claim_public_inputs_to_vec,
    generated::transfer_claim::TransferClaimPublicInputs
);

struct PublicInputParts {
    chain_id: Element,
    bridge_address: Element,
    recent_root: Element,
    input_nullifiers: [Element; 2],
    output_commitments: [Element; 2],
    nonce_hash: Element,
    user_encrypted_key_hash: Element,
    recipient_encrypted_key_hash: Element,
    sender_encrypted_note: [Element; 5],
    recipient_encrypted_note: [Element; 5],
    sender_chain_encrypted_key: [Element; 3],
    recipient_chain_encrypted_key: [Element; 3],
    chain_public_key: [Element; 2],
    token: Element,
    burn_recipient: Element,
    value: Element,
    mint_from: Element,
    receive_prefix: Element,
}

fn canonical_public_inputs_to_vec(inputs: &PublicInputParts) -> Vec<[u8; 32]> {
    let mut out = Vec::with_capacity(33);
    out.extend(
        [
            inputs.chain_id,
            inputs.bridge_address,
            inputs.recent_root,
            inputs.input_nullifiers[0],
            inputs.input_nullifiers[1],
            inputs.output_commitments[0],
            inputs.output_commitments[1],
            inputs.nonce_hash,
            inputs.user_encrypted_key_hash,
            inputs.recipient_encrypted_key_hash,
        ]
        .into_iter()
        .map(Element::to_be_bytes),
    );
    out.extend(
        inputs
            .sender_encrypted_note
            .iter()
            .copied()
            .map(Element::to_be_bytes),
    );
    out.extend(
        inputs
            .recipient_encrypted_note
            .iter()
            .copied()
            .map(Element::to_be_bytes),
    );
    out.extend(
        inputs
            .sender_chain_encrypted_key
            .iter()
            .copied()
            .map(Element::to_be_bytes),
    );
    out.extend(
        inputs
            .recipient_chain_encrypted_key
            .iter()
            .copied()
            .map(Element::to_be_bytes),
    );
    out.extend(
        [
            inputs.chain_public_key[0],
            inputs.chain_public_key[1],
            inputs.token,
            inputs.burn_recipient,
            inputs.value,
            inputs.mint_from,
            inputs.receive_prefix,
        ]
        .into_iter()
        .map(Element::to_be_bytes),
    );
    out
}

/// Canonical operation-to-circuit mapping.
#[must_use]
pub const fn supported_circuits() -> [PrivacyCircuit; 4] {
    [
        PrivacyCircuit::Mint,
        PrivacyCircuit::Burn,
        PrivacyCircuit::TransferSend,
        PrivacyCircuit::TransferClaim,
    ]
}
