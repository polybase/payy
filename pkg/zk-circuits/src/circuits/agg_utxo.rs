use element::Element;
use zk_primitives::{AggUtxo, AggUtxoProof, AggUtxoPublicInput, UtxoProofBundleWithMerkleProofs};

use crate::circuits::Proof;
use crate::{Prove, Result, Verify};

use super::UTXO_VERIFICATION_KEY;
use super::conversions::impl_circuit_proof_conversions;
use super::generated::agg_utxo::{
    AggUtxoInput, AggUtxoPublicInputs as CircuitAggUtxoPublicInputs,
    Aggutxoproofinput as CircuitAggUtxoProofInput,
};

impl From<UtxoProofBundleWithMerkleProofs> for CircuitAggUtxoProofInput {
    fn from(value: UtxoProofBundleWithMerkleProofs) -> Self {
        let UtxoProofBundleWithMerkleProofs {
            utxo_proof,
            input_merkle_paths,
            output_merkle_paths,
        } = value;

        let proof_fields = utxo_proof.proof.to_fields();

        let input_merkle_paths = input_merkle_paths.map(|path| {
            let siblings = path.siblings;
            std::array::from_fn(|idx| siblings[idx])
        });

        let output_merkle_paths = output_merkle_paths.map(|path| {
            let siblings = path.siblings;
            std::array::from_fn(|idx| siblings[idx])
        });

        Self {
            proof: std::array::from_fn(|idx| proof_fields[idx]),
            utxo_kind: utxo_proof.kind().to_element(),
            input_merkle_paths,
            output_merkle_paths,
            input_commitments: utxo_proof.public_inputs.input_commitments,
            output_commitments: utxo_proof.public_inputs.output_commitments,
        }
    }
}

impl From<AggUtxo> for AggUtxoInput {
    fn from(agg_utxo: AggUtxo) -> Self {
        let commit_hash = agg_utxo.commit_hash();
        let messages = agg_utxo.messages();

        let AggUtxo {
            proofs,
            old_root,
            new_root,
        } = agg_utxo;

        let verification_key =
            std::array::from_fn(|idx| Element::from(UTXO_VERIFICATION_KEY.0[idx]));

        Self {
            verification_key,
            proofs: proofs.map(CircuitAggUtxoProofInput::from),
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
    AggUtxoPublicInput,
    CircuitAggUtxoPublicInputs,
    AggUtxoProof,
    Proof<CircuitAggUtxoPublicInputs>,
    [
        verification_key_hash,
        old_root,
        new_root,
        commit_hash,
        messages
    ]
);

#[async_trait::async_trait]
impl Prove for AggUtxo {
    type Proof = AggUtxoProof;

    async fn prove(
        &self,
        bb_backend: &dyn barretenberg_interface::BbBackend,
    ) -> Result<Self::Proof> {
        let input = AggUtxoInput::from(self.clone());
        let proof = <AggUtxoInput as Prove>::prove(&input, bb_backend).await?;
        Ok(proof.into())
    }
}

#[async_trait::async_trait]
impl Verify for AggUtxoProof {
    async fn verify(&self, bb_backend: &dyn barretenberg_interface::BbBackend) -> Result<()> {
        let proof = Proof::from(self.clone());
        proof.verify(bb_backend).await
    }
}
