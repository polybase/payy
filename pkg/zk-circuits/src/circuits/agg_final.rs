use element::Element;
use zk_primitives::{AggFinal, AggFinalProof, AggFinalPublicInput, OracleProofBytes};

use super::generated::agg_final::{
    AggFinalInput, AggFinalPublicInputs as CircuitAggFinalPublicInputs,
};
use crate::circuits::Proof;
use crate::{Prove, Result, Verify};

fn messages_vec_to_array(messages: Vec<Element>) -> [Element; 1000] {
    let mut array = [Element::ZERO; 1000];
    let len = messages.len().min(array.len());
    array[..len].copy_from_slice(&messages[..len]);
    array
}

impl From<AggFinal> for AggFinalInput {
    fn from(agg_final: AggFinal) -> Self {
        let AggFinal { proof } = agg_final;
        let proof_fields = proof.proof.to_fields();

        Self {
            verification_key: std::array::from_fn(|idx| {
                Element::from(super::generated::agg_agg::VERIFICATION_KEY[idx])
            }),
            verification_key_hash: Element::from(*super::generated::agg_agg::VERIFICATION_KEY_HASH),
            proof: std::array::from_fn(|idx| proof_fields[idx]),
            commit_hash: proof.public_inputs.commit_hash,
            old_root: proof.public_inputs.old_root,
            new_root: proof.public_inputs.new_root,
            messages: proof.public_inputs.messages,
        }
    }
}

impl From<CircuitAggFinalPublicInputs> for AggFinalPublicInput {
    fn from(value: CircuitAggFinalPublicInputs) -> Self {
        Self {
            old_root: value.old_root,
            new_root: value.new_root,
            commit_hash: value.commit_hash,
            messages: value.messages.to_vec(),
        }
    }
}

impl From<AggFinalPublicInput> for CircuitAggFinalPublicInputs {
    fn from(value: AggFinalPublicInput) -> Self {
        Self {
            old_root: value.old_root,
            new_root: value.new_root,
            commit_hash: value.commit_hash,
            messages: messages_vec_to_array(value.messages),
        }
    }
}

impl From<Proof<CircuitAggFinalPublicInputs>> for AggFinalProof {
    fn from(value: Proof<CircuitAggFinalPublicInputs>) -> Self {
        Self {
            proof: OracleProofBytes(value.proof),
            public_inputs: value.public_inputs.into(),
            kzg: vec![],
        }
    }
}

impl From<AggFinalProof> for Proof<CircuitAggFinalPublicInputs> {
    fn from(value: AggFinalProof) -> Self {
        Self {
            proof: value.proof.0,
            public_inputs: value.public_inputs.into(),
        }
    }
}

#[async_trait::async_trait]
impl Prove for AggFinal {
    type Proof = AggFinalProof;

    async fn prove(
        &self,
        bb_backend: &dyn barretenberg_interface::BbBackend,
    ) -> Result<Self::Proof> {
        let input = AggFinalInput::from(self.clone());
        let proof = input.prove(bb_backend).await?;
        Ok(proof.into())
    }
}

#[async_trait::async_trait]
impl Verify for AggFinalProof {
    async fn verify(&self, bb_backend: &dyn barretenberg_interface::BbBackend) -> Result<()> {
        let proof = Proof::from(self.clone());
        proof.verify(bb_backend).await
    }
}
