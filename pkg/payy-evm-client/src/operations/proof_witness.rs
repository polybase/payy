#![allow(clippy::too_many_arguments)]

use contextful::{ErrorContextExt, ResultContextExt};
use element::Element;
use payy_evm_client_interface::{B256, Result};
use payy_evm_client_prover_interface::{PrivacyCircuit, ProveRequest};
use zk_circuits::circuits::ProofInputs;
use zk_circuits::execute::execute_program_and_decode;

use super::proof::ProvenCall;
use crate::client::PrivacyNamespace;
use crate::util::element_to_b256;

pub(super) async fn prove_witness<T>(
    client: &PrivacyNamespace,
    circuit: PrivacyCircuit,
    tx_commitment: Element,
    recent_root: Element,
    input_commitments: Vec<Element>,
    input_nullifiers: Vec<Element>,
    output_commitments: Vec<Element>,
    user_encrypted_key: [B256; 4],
    recipient_encrypted_key: [B256; 4],
    witness: T,
) -> Result<ProvenCall>
where
    T: ProofInputs,
{
    let input_map = witness.input_map();
    let execution = execute_program_and_decode(witness.compiled_program(), &input_map, false)
        .map_err(|err| {
            payy_evm_client_interface::Error::Internal(
                std::io::Error::other(err.to_string())
                    .context("execute circuit witness")
                    .into(),
            )
        })?;
    let witness =
        bincode::serde::encode_to_vec(&execution.witness_stack, bincode::config::legacy())
            .context("encode circuit witness")?;
    let output = client
        .inner
        .prover
        .prove(ProveRequest { circuit, witness })
        .await
        .context("prove privacy circuit")?;
    Ok(ProvenCall {
        verification_key_hash: output.verification_key_hash,
        proof: output.proof,
        public_inputs: output.public_inputs,
        tx_commitment: element_to_b256(tx_commitment),
        recent_root: element_to_b256(recent_root),
        input_commitments: input_commitments
            .into_iter()
            .filter(|commitment| !commitment.is_zero())
            .map(element_to_b256)
            .collect(),
        input_nullifiers: input_nullifiers
            .into_iter()
            .filter(|nullifier| !nullifier.is_zero())
            .map(element_to_b256)
            .collect(),
        output_commitments: output_commitments
            .into_iter()
            .filter(|commitment| !commitment.is_zero())
            .map(element_to_b256)
            .collect(),
        user_encrypted_key,
        recipient_encrypted_key,
    })
}
