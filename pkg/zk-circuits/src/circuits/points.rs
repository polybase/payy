// lint-long-file-override allow-max-lines=300
use zk_primitives::{Points, PointsProof, PointsPublicInput};

use crate::circuits::Proof;

use super::conversions::impl_circuit_proof_conversions;
use super::generated::points::{PointsInput, PointsPublicInputs as CircuitPointsPublicInputs};
use super::generated::submodules::common::Note as CommonNote;

impl From<Points> for PointsInput {
    fn from(points: Points) -> Self {
        let Points {
            secret_keys,
            notes,
            timestamp,
            address,
        } = points;
        let value = notes.each_ref().map(|note| note.value).into_iter().sum();
        let commitments = notes.each_ref().map(zk_primitives::Note::commitment);

        PointsInput {
            secret_keys,
            notes: notes.map(CommonNote::from),
            timestamp,
            address,
            value,
            hash: hash::hash_merge([timestamp, address]),
            commitments,
        }
    }
}

impl_circuit_proof_conversions!(
    PointsPublicInput,
    CircuitPointsPublicInputs,
    PointsProof,
    Proof<CircuitPointsPublicInputs>,
    [value, timestamp, hash, commitments]
);
