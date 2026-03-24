use lazy_static::lazy_static;
use zk_primitives::{Utxo, UtxoProof, UtxoPublicInput};

use crate::circuits::Proof;
use crate::verify::{VerificationKey, VerificationKeyHash};
use crate::{Prove, Result, Verify};

use super::generated::submodules::common::{Inputnote as CommonInputNote, Note as CommonNote};
use super::generated::utxo::{UtxoInput, UtxoPublicInputs as CircuitUtxoPublicInputs};

lazy_static! {
    pub static ref UTXO_VERIFICATION_KEY: VerificationKey =
        VerificationKey(super::generated::utxo::VERIFICATION_KEY.clone());
    pub static ref UTXO_VERIFICATION_KEY_HASH: VerificationKeyHash =
        VerificationKeyHash(*super::generated::utxo::VERIFICATION_KEY_HASH);
}

impl From<Utxo> for UtxoInput {
    fn from(utxo: Utxo) -> Self {
        let messages = utxo.messages();
        let commitments = utxo.leaf_elements();
        let Utxo {
            input_notes,
            output_notes,
            ..
        } = utxo;

        UtxoInput {
            input_notes: input_notes.map(CommonInputNote::from),
            output_notes: output_notes.map(CommonNote::from),
            pmessage4: messages[4],
            commitments,
            messages,
        }
    }
}

#[async_trait::async_trait]
impl Prove for Utxo {
    type Proof = UtxoProof;

    async fn prove(
        &self,
        bb_backend: &dyn barretenberg_interface::BbBackend,
    ) -> Result<Self::Proof> {
        let input = UtxoInput::from(self.clone());
        let proof = input.prove(bb_backend).await?;
        Ok(UtxoProof::from(proof))
    }
}

impl From<CircuitUtxoPublicInputs> for UtxoPublicInput {
    fn from(value: CircuitUtxoPublicInputs) -> Self {
        Self {
            input_commitments: [value.commitments[0], value.commitments[1]],
            output_commitments: [value.commitments[2], value.commitments[3]],
            messages: value.messages,
        }
    }
}

impl From<UtxoPublicInput> for CircuitUtxoPublicInputs {
    fn from(value: UtxoPublicInput) -> Self {
        Self {
            commitments: [
                value.input_commitments[0],
                value.input_commitments[1],
                value.output_commitments[0],
                value.output_commitments[1],
            ],
            messages: value.messages,
        }
    }
}

impl From<Proof<CircuitUtxoPublicInputs>> for UtxoProof {
    fn from(value: Proof<CircuitUtxoPublicInputs>) -> Self {
        Self {
            proof: zk_primitives::ProofBytes(value.proof),
            public_inputs: value.public_inputs.into(),
        }
    }
}

impl From<UtxoProof> for Proof<CircuitUtxoPublicInputs> {
    fn from(value: UtxoProof) -> Self {
        Self {
            proof: value.proof.0,
            public_inputs: value.public_inputs.into(),
        }
    }
}

#[async_trait::async_trait]
impl Verify for UtxoProof {
    async fn verify(&self, bb_backend: &dyn barretenberg_interface::BbBackend) -> Result<()> {
        let proof = Proof::from(self.clone());
        proof.verify(bb_backend).await
    }
}
