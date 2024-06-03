use crate::{
    chips::aggregation::snark::Snark,
    data::{AggregateAgg, ParameterSet, SnarkWitness},
    evm_verifier, CircuitKind,
};
use borsh::{BorshDeserialize, BorshSerialize};
use smirk::Element;
use wire_message::{wire_message, WireMessage};

use super::fs::{load_file, load_witness, save_file, save_witness};

pub fn create_or_load_agg_agg_utxo_snark(params: ParameterSet, snarks: [Snark; 2]) -> Snark {
    load_witness("agg_utxo_agg")
        .map(|sw| match sw {
            SnarkWitness::V1(sw) => sw,
        })
        .map(|sw| sw.to_snark(CircuitKind::AggAgg.vk(), params))
        .unwrap_or_else(|| {
            // Currently we can only do 1 for the Ethereum verifier as 2 creates a "too large" verifier (25,137 bytes) where
            // the max limit is 24,576 bytes (we are so close, we might be able to get this to fit!)
            let aggregate_agg_agg = AggregateAgg::new(snarks);
            let snark = aggregate_agg_agg.snark(params).unwrap();

            save_witness("agg_utxo_agg", &SnarkWitness::V1(snark.to_witness()));
            snark
        })
}

pub fn create_or_load_agg_agg_final_snark(params: ParameterSet, snark: Snark) -> Snark {
    load_witness("agg_agg_final")
        .map(|sw| match sw {
            SnarkWitness::V1(sw) => sw,
        })
        .map(|sw| {
            sw.to_snark(
                &AggregateAgg::<1>::new([snark.clone()]).keygen(params).1,
                params,
            )
        })
        .unwrap_or_else(|| {
            // Currently we can only do 1 for the Ethereum verifier as 2 creates a "too large" verifier (25,137 bytes) where
            // the max limit is 24,576 bytes (we are so close, we might be able to get this to fit!)
            let aggregate_agg_agg = AggregateAgg::<1>::new([snark]);
            let snark = aggregate_agg_agg.snark(params).unwrap();

            save_witness("agg_agg_final", &SnarkWitness::V1(snark.to_witness()));
            snark
        })
}

#[derive(Clone, Debug)]
#[wire_message]
pub enum EvmProof {
    V1(EvmProofV1),
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct EvmProofV1 {
    pub proof: Vec<u8>,
    pub agg_instances: Vec<Element>,
    pub old_root: Element,
    pub new_root: Element,
    pub utxo_inputs: Vec<Element>,
}

impl WireMessage for EvmProof {
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        match self {
            Self::V1(_) => Err(Self::max_version_error()),
        }
    }

    fn version(&self) -> u64 {
        match self {
            Self::V1(_) => 1,
        }
    }
}

pub fn create_or_load_agg_agg_final_evm_proof(
    params: ParameterSet,
    agg_agg_utxo: Snark,
) -> EvmProof {
    load_file("agg_agg_final_evm_proof").unwrap_or_else(|| {
        let aggregate_agg_agg = AggregateAgg::<1>::new([agg_agg_utxo]);
        let inputs = aggregate_agg_agg.public_inputs();
        let (pk, _) = aggregate_agg_agg.keygen(params);

        let proof =
            evm_verifier::gen_proof(params, &pk, aggregate_agg_agg.clone(), &[&inputs]).unwrap();

        let evm_proof = EvmProofV1 {
            proof,
            agg_instances: aggregate_agg_agg
                .agg_instances()
                .iter()
                .cloned()
                .map(From::from)
                .collect(),
            old_root: (*aggregate_agg_agg.old_root()).into(),
            new_root: (*aggregate_agg_agg.new_root()).into(),
            utxo_inputs: aggregate_agg_agg
                .utxo_values()
                .into_iter()
                .map(From::from)
                .collect::<Vec<_>>(),
        };
        let evm_proof = EvmProof::V1(evm_proof);

        save_file("agg_agg_final_evm_proof", &evm_proof);

        evm_proof
    })
}
