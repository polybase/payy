use crate::{
    backend::DefaultBackend,
    circuits::get_bytecode_from_program,
    prove::prove,
    traits::{Prove, Verify},
    util::write_to_temp_file,
    verify::verify,
    Result,
};
use element::Base;
use lazy_static::lazy_static;
use noirc_abi::{input_parser::InputValue, InputMap};
use noirc_artifacts::program::ProgramArtifact;
use noirc_driver::CompiledProgram;
use std::path::PathBuf;
use zk_primitives::{
    bytes_to_elements, Points, PointsProof, PointsProofBytes, PointsPublicInput, ToBytes,
};

use super::note::BNote;

const PROGRAM: &str = include_str!("../../../../fixtures/programs/points.json");
const KEY: &[u8] = include_bytes!("../../../../fixtures/keys/points_key");

lazy_static! {
    static ref PROGRAM_ARTIFACT: ProgramArtifact = serde_json::from_str(PROGRAM).unwrap();
    static ref PROGRAM_COMPILED: CompiledProgram = CompiledProgram::from(PROGRAM_ARTIFACT.clone());
    static ref PROGRAM_PATH: PathBuf = write_to_temp_file(PROGRAM.as_bytes(), ".json");
    static ref BYTECODE: Vec<u8> = get_bytecode_from_program(PROGRAM);
}

const POINTS_PUBLIC_INPUTS_COUNT: usize = 13;
const POINTS_PROOF_SIZE: usize = 440;

impl Prove for Points {
    type Proof = PointsProof;
    type Result<Proof> = Result<Proof>;

    fn prove(&self) -> Self::Result<Self::Proof> {
        let inputs = InputMap::from(PointsInput::from(self));

        let proof_bytes = prove::<DefaultBackend>(
            &PROGRAM_COMPILED,
            PROGRAM.as_bytes(),
            &BYTECODE,
            &inputs,
            false,
            false,
        )?;

        // Slice off the public inputs
        let public_inputs = proof_bytes[..POINTS_PUBLIC_INPUTS_COUNT * 32].to_vec();
        let public_inputs = bytes_to_elements(&public_inputs);
        let raw_proof = proof_bytes[POINTS_PUBLIC_INPUTS_COUNT * 32..].to_vec();

        assert_eq!(
            raw_proof.len(),
            POINTS_PROOF_SIZE * 32,
            "Proof must be {POINTS_PROOF_SIZE} elements of 32 bytes"
        );

        // Convert the proof bytes to a UtxoProof
        let proof = PointsProof {
            proof: PointsProofBytes(raw_proof),
            public_inputs: PointsPublicInput {
                timestamp: public_inputs[0],
                value: public_inputs[1],
                hash: public_inputs[2],
                commitments: public_inputs[3..].try_into().unwrap(),
            },
        };

        Ok(proof)
    }
}

impl Verify for PointsProof {
    fn verify(&self) -> Result<()> {
        verify::<DefaultBackend>(KEY, &self.to_bytes(), false)
    }
}

struct PointsInput {
    secret_keys: [Base; 10],
    notes: [BNote; 10],
    timestamp: Base,
    address: Base,
    value: Base,
    hash: Base,
    commitments: [Base; 10],
}

impl From<&Points> for PointsInput {
    fn from(points: &Points) -> Self {
        PointsInput {
            secret_keys: points
                .secret_keys
                .iter()
                .map(|e| e.to_base())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            notes: points
                .notes
                .iter()
                .map(BNote::from)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            timestamp: points.timestamp.to_base(),
            address: points.address.to_base(),
            value: points.value().to_base(),
            hash: points.hash().to_base(),
            commitments: points
                .notes
                .iter()
                .map(|e| e.commitment().to_base())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl From<PointsInput> for InputMap {
    fn from(input: PointsInput) -> Self {
        let mut map = InputMap::new();
        map.insert(
            "notes".to_string(),
            InputValue::Vec(input.notes.map(InputValue::from).to_vec()),
        );
        map.insert(
            "secret_keys".to_string(),
            InputValue::Vec(input.secret_keys.map(InputValue::Field).to_vec()),
        );
        map.insert("address".to_string(), InputValue::Field(input.address));
        map.insert("timestamp".to_string(), InputValue::Field(input.timestamp));
        map.insert("value".to_string(), InputValue::Field(input.value));
        map.insert("hash".to_string(), InputValue::Field(input.hash));
        map.insert(
            "commitments".to_string(),
            InputValue::Vec(input.commitments.map(InputValue::Field).to_vec()),
        );
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{Prove, Verify};
    use element::Element;
    use zk_primitives::{get_address_for_private_key, Note, Points};

    #[test]
    fn test_points_prove_and_verify() -> Result<()> {
        // Create a test Points instance
        let secret_key = Element::new(101);
        let address = get_address_for_private_key(secret_key);

        let mut notes = (1..11)
            .map(|i| Note {
                value: Element::new(i as u64),
                address,
                kind: Element::new(1),
                psi: Element::new(i as u64),
            })
            .collect::<Vec<_>>();

        let mut secret_keys = [secret_key; 10];

        notes[0] = Note::padding_note();
        secret_keys[0] = Element::ZERO;

        let points = Points {
            secret_keys,
            notes: notes.try_into().unwrap(),
            timestamp: Element::new(1234567890u64),
            address,
        };

        // Generate a proof
        let proof = points.prove()?;

        // Verify the proof
        assert!(proof.verify().is_ok(), "Proof verification failed");

        Ok(())
    }
}
