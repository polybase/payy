use super::{UTXO_VERIFICATION_KEY, UTXO_VERIFICATION_KEY_HASH};
use crate::backend::DefaultBackend;
use crate::circuits::get_bytecode_from_program;
use crate::prove::prove;
use crate::traits::{Prove, Verify};
use crate::util::write_to_temp_file;
use crate::verify::{verify, VerificationKey, VerificationKeyHash};
use crate::Result;
use core::iter::Iterator;
use element::{Base, Element};
use lazy_static::lazy_static;
use noirc_abi::{input_parser::InputValue, InputMap};
use noirc_artifacts::program::ProgramArtifact;
use noirc_driver::CompiledProgram;
use std::collections::BTreeMap;
use std::path::PathBuf;
use zk_primitives::{
    bytes_to_elements, AggUtxo, AggUtxoProof, AggUtxoProofBytes, AggUtxoPublicInput, MerklePath,
    ToBytes, UtxoProofBundleWithMerkleProofs,
};

const PROGRAM: &str = include_str!("../../../../fixtures/programs/agg_test.json");
const KEY: &[u8] = include_bytes!("../../../../fixtures/keys/agg_test_key");
const KEY_FIELDS: &[u8] = include_bytes!("../../../../fixtures/keys/agg_test_key_fields.json");

lazy_static! {
    static ref PROGRAM_ARTIFACT: ProgramArtifact = serde_json::from_str(PROGRAM).unwrap();
    static ref PROGRAM_COMPILED: CompiledProgram = CompiledProgram::from(PROGRAM_ARTIFACT.clone());
    static ref PROGRAM_PATH: PathBuf = write_to_temp_file(PROGRAM.as_bytes(), ".json");
    static ref BYTECODE: Vec<u8> = get_bytecode_from_program(PROGRAM);
    pub static ref AGG_TEST_VERIFICATION_KEY: VerificationKey =
        VerificationKey(serde_json::from_slice(KEY_FIELDS).unwrap());
    pub static ref AGG_TEST_VERIFICATION_KEY_HASH: VerificationKeyHash = VerificationKeyHash(
        bn254_blackbox_solver::poseidon_hash(&AGG_TEST_VERIFICATION_KEY.0, false).unwrap()
    );
}

const AGG_TEST_PUBLIC_INPUTS_COUNT: usize = 0;

#[derive(Debug, Clone)]
pub struct AggTestInput {
    pub proof: [Base; 507],
    pub public_inputs: [Base; 10],
}

impl Prove for AggTestInput {
    type Proof = AggTestProof;
    type Result<Proof> = Result<Proof>;

    fn prove(&self) -> Self::Result<Self::Proof> {
        let inputs = InputMap::from(self.clone());

        println!("proof=[");
        for i in &self.proof {
            println!(" \"0x{}\",", element::Element::from_base(*i).to_hex());
        }
        println!("]");

        println!("");

        // println!(
        //     "AGG_TEST_VERIFICATION_KEY_HASH: {}",
        //     Element::from_base(AGG_TEST_VERIFICATION_KEY_HASH.0).to_hex()
        // );
        println!("public_inputs=[");
        for input in self.public_inputs {
            println!(" \"0x{}\",", Element::from_base(input).to_hex());
        }
        println!("]");

        let proof_bytes = prove::<DefaultBackend>(
            &PROGRAM_COMPILED,
            PROGRAM.as_bytes(),
            &BYTECODE,
            KEY,
            &inputs,
            true,
            false,
        )?;

        // Slice the first 8, 32 byte chunks as the public inputs
        let public_inputs = proof_bytes[..AGG_TEST_PUBLIC_INPUTS_COUNT * 32].to_vec();
        let public_inputs = bytes_to_elements(&public_inputs);
        let raw_proof = proof_bytes[AGG_TEST_PUBLIC_INPUTS_COUNT * 32..].to_vec();

        assert_eq!(
            public_inputs.len(),
            AGG_TEST_PUBLIC_INPUTS_COUNT,
            "Public inputs must be {} elements",
            AGG_TEST_PUBLIC_INPUTS_COUNT
        );

        assert_eq!(
            raw_proof.len(),
            507 * 32,
            "Proof must be 507 elements of 32 bytes, got: {}",
            raw_proof.len() / 32
        );

        Ok(AggTestProof {
            proof: raw_proof,
            public_inputs,
        })
    }
}

pub struct AggTestProof {
    proof: Vec<u8>,
    public_inputs: Vec<element::Element>,
}

impl AggTestProof {
    pub fn public_input_bytes(&self) -> Vec<u8> {
        self.public_inputs
            .iter()
            .flat_map(|e| e.to_be_bytes())
            .collect::<Vec<u8>>()
    }
}

impl ToBytes for AggTestProof {
    /// Convert the UtxoProof to a UtxoProofFields
    fn to_bytes(&self) -> Vec<u8> {
        // TODO: move to impl detail of proving backend
        let pi = self.public_input_bytes();
        let proof = self.proof.clone();
        [pi.as_slice(), proof.as_slice()].concat()
    }
}

impl Verify for AggTestProof {
    fn verify(&self) -> Result<()> {
        verify::<DefaultBackend>(KEY, &self.to_bytes(), false)
    }
}

impl From<AggTestInput> for InputMap {
    fn from(value: AggTestInput) -> Self {
        let mut map = InputMap::new();

        // Should be static
        map.insert(
            "verification_key".to_owned(),
            InputValue::Vec(
                UTXO_VERIFICATION_KEY
                    .0
                    .iter()
                    .cloned()
                    .map(InputValue::Field)
                    .collect(),
            ),
        );
        // map.insert(
        //     "verification_key_hash".to_owned(),
        //     InputValue::Field(UTXO_VERIFICATION_KEY_HASH.0),
        // );

        map.insert(
            "proof".to_owned(),
            InputValue::Vec(value.proof.map(InputValue::Field).to_vec()),
        );
        map.insert(
            "public_inputs".to_owned(),
            InputValue::Vec(value.public_inputs.map(InputValue::Field).to_vec()),
        );

        map
    }
}

#[derive(Debug, Clone)]
pub struct AggTestProofInput {}
