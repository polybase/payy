use super::note::{BInputNote, BNote};
use crate::{
    backend::DefaultBackend,
    circuits::get_bytecode_from_program,
    prove::prove,
    traits::{Prove, Verify},
    util::write_to_temp_file,
    verify::{verify, VerificationKey, VerificationKeyHash},
    Result,
};
use element::Base;
use lazy_static::lazy_static;
use noirc_abi::{input_parser::InputValue, InputMap};
use noirc_artifacts::program::ProgramArtifact;
use noirc_driver::CompiledProgram;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zk_primitives::{
    bytes_to_elements, ToBytes, Utxo, UtxoProof, UtxoProofBytes, UtxoPublicInput, UTXO_PROOF_SIZE,
    UTXO_PUBLIC_INPUTS_COUNT,
};

const PROGRAM: &str = include_str!("../../../../fixtures/programs/utxo.json");
const KEY: &[u8] = include_bytes!("../../../../fixtures/keys/utxo_key");
const KEY_FIELDS: &[u8] = include_bytes!("../../../../fixtures/keys/utxo_key_fields.json");

lazy_static! {
    static ref PROGRAM_ARTIFACT: ProgramArtifact = serde_json::from_str(PROGRAM).unwrap();
    static ref PROGRAM_COMPILED: CompiledProgram = CompiledProgram::from(PROGRAM_ARTIFACT.clone());
    static ref PROGRAM_PATH: PathBuf = write_to_temp_file(PROGRAM.as_bytes(), ".json");
    static ref BYTECODE: Vec<u8> = get_bytecode_from_program(PROGRAM);
    pub static ref UTXO_VERIFICATION_KEY: VerificationKey = {
        let mut fields = serde_json::from_slice::<Vec<Base>>(KEY_FIELDS).unwrap();
        VerificationKey(fields)
    };
    pub static ref UTXO_VERIFICATION_KEY_HASH: VerificationKeyHash = VerificationKeyHash(
        bn254_blackbox_solver::poseidon_hash(&UTXO_VERIFICATION_KEY.0, false).unwrap()
    );
}

impl Prove for Utxo {
    type Proof = UtxoProof;
    type Result<Proof> = Result<Proof>;

    fn prove(&self) -> Self::Result<Self::Proof> {
        println!("=====================");
        println!("Proving UTXO...");
        println!("=====================");

        let utxo_input = UtxoInput::from(self);
        println!(
            "inputs {}",
            serde_json::to_string_pretty(&utxo_input).unwrap()
        );
        let inputs = InputMap::from(utxo_input);

        let proof_bytes = prove::<DefaultBackend>(
            &PROGRAM_COMPILED,
            PROGRAM.as_bytes(),
            &BYTECODE,
            KEY,
            &inputs,
            true,
            false,
        )?;

        // Slice the first 6, 32 byte chunks as the public inputs
        let public_inputs = proof_bytes[..UTXO_PUBLIC_INPUTS_COUNT * 32].to_vec();
        let public_inputs = bytes_to_elements(&public_inputs);

        let raw_proof = proof_bytes[UTXO_PUBLIC_INPUTS_COUNT * 32..].to_vec();

        assert_eq!(
            raw_proof.len(),
            UTXO_PROOF_SIZE * 32,
            "Proof must be {UTXO_PROOF_SIZE} elements of 32 bytes"
        );

        // Convert the proof bytes to a UtxoProof
        let proof = UtxoProof {
            proof: UtxoProofBytes(raw_proof),
            public_inputs: UtxoPublicInput {
                input_commitments: [public_inputs[0], public_inputs[1]],
                output_commitments: [public_inputs[2], public_inputs[3]],
                messages: [
                    public_inputs[4],
                    public_inputs[5],
                    public_inputs[6],
                    public_inputs[7],
                    public_inputs[8],
                    public_inputs[9],
                ],
            },
        };

        Ok(proof)
    }
}

impl Verify for UtxoProof {
    fn verify(&self) -> Result<()> {
        let bytes = self.to_bytes();
        verify::<crate::backend::DefaultBackend>(KEY, &bytes, false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UtxoInput {
    input_notes: [BInputNote; 2],
    output_notes: [BNote; 2],
    pmessage4: Base,
    commitments: [Base; 4],
    messages: [Base; 6],
}

impl From<&Utxo> for UtxoInput {
    fn from(utxo: &Utxo) -> Self {
        UtxoInput {
            input_notes: utxo
                .input_notes
                .iter()
                .map(BInputNote::from)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            output_notes: utxo
                .output_notes
                .iter()
                .map(BNote::from)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            pmessage4: utxo.messages()[4].to_base(),
            commitments: [
                utxo.input_notes[0].note.commitment().to_base(),
                utxo.input_notes[1].note.commitment().to_base(),
                utxo.output_notes[0].commitment().to_base(),
                utxo.output_notes[1].commitment().to_base(),
            ],
            messages: utxo.messages().map(|e| e.to_base()),
        }
    }
}

impl From<UtxoInput> for InputMap {
    fn from(utxo: UtxoInput) -> Self {
        let mut map = InputMap::new();

        map.insert(
            "input_notes".to_owned(),
            InputValue::Vec(utxo.input_notes.map(InputValue::from).to_vec()),
        );
        map.insert(
            "output_notes".to_owned(),
            InputValue::Vec(utxo.output_notes.map(InputValue::from).to_vec()),
        );

        map.insert("pmessage4".to_owned(), InputValue::Field(utxo.pmessage4));

        map.insert(
            "commitments".to_owned(),
            InputValue::Vec(utxo.commitments.map(InputValue::Field).to_vec()),
        );

        map.insert(
            "messages".to_owned(),
            InputValue::Vec(utxo.messages.map(InputValue::Field).to_vec()),
        );

        map
    }
}
