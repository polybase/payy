use super::conversions::impl_circuit_proof_conversions;
use super::generated::agg_agg::{
    AggAggInput, AggAggPublicInputs as CircuitAggAggPublicInputs, Aggproof as CircuitAggProofInput,
};
use crate::circuits::Proof;
use crate::{Prove, Result, Verify};
use element::Element;
use zk_primitives::{AggAgg, AggAggProof, AggAggPublicInput, AggProof};

impl From<AggProof> for CircuitAggProofInput {
    fn from(value: AggProof) -> Self {
        match value {
            AggProof::AggUtxo(proof) => {
                let proof_fields = proof.proof.to_fields();

                Self {
                    proof: std::array::from_fn(|idx| proof_fields[idx]),
                    old_root: proof.public_inputs.old_root,
                    new_root: proof.public_inputs.new_root,
                    commit_hash: proof.public_inputs.commit_hash,
                    messages: proof.public_inputs.messages,
                    verification_key: std::array::from_fn(|idx| {
                        Element::from(super::generated::agg_utxo::VERIFICATION_KEY[idx])
                    }),
                    verification_key_hash: Element::from(
                        *super::generated::agg_utxo::VERIFICATION_KEY_HASH,
                    ),
                }
            }
            AggProof::AggAgg(proof) => {
                let proof_fields = proof.proof.to_fields();

                Self {
                    proof: std::array::from_fn(|idx| proof_fields[idx]),
                    old_root: proof.public_inputs.old_root,
                    new_root: proof.public_inputs.new_root,
                    commit_hash: proof.public_inputs.commit_hash,
                    messages: proof.public_inputs.messages,
                    verification_key: std::array::from_fn(|idx| {
                        Element::from(super::generated::agg_agg::VERIFICATION_KEY[idx])
                    }),
                    verification_key_hash: Element::from(
                        *super::generated::agg_agg::VERIFICATION_KEY_HASH,
                    ),
                }
            }
        }
    }
}

impl From<AggAgg> for AggAggInput {
    fn from(agg_agg: AggAgg) -> Self {
        let old_root = agg_agg.old_root();
        let new_root = agg_agg.new_root();
        let commit_hash = agg_agg.commit_hash();
        let messages = agg_agg.messages();
        let AggAgg { proofs } = agg_agg;

        Self {
            proofs: proofs.map(CircuitAggProofInput::from),
            verification_key_hash: [
                Element::from(*super::generated::agg_utxo::VERIFICATION_KEY_HASH),
                Element::from(*super::generated::agg_agg::VERIFICATION_KEY_HASH),
            ],
            old_root,
            new_root,
            commit_hash,
            messages,
        }
    }
}

impl_circuit_proof_conversions!(
    AggAggPublicInput,
    CircuitAggAggPublicInputs,
    AggAggProof,
    Proof<CircuitAggAggPublicInputs>,
    [
        verification_key_hash,
        old_root,
        new_root,
        commit_hash,
        messages
    ],
    [kzg: vec![]]
);

#[async_trait::async_trait]
impl Prove for AggAgg {
    type Proof = AggAggProof;

    async fn prove(
        &self,
        bb_backend: &dyn barretenberg_interface::BbBackend,
    ) -> Result<Self::Proof> {
        let input = AggAggInput::from(self.clone());
        let proof = input.prove(bb_backend).await?;
        Ok(proof.into())
    }
}

#[async_trait::async_trait]
impl Verify for AggAggProof {
    async fn verify(&self, bb_backend: &dyn barretenberg_interface::BbBackend) -> Result<()> {
        let proof = Proof::from(self.clone());
        proof.verify(bb_backend).await
    }
}
