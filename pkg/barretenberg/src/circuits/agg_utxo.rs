use super::{UTXO_VERIFICATION_KEY, UTXO_VERIFICATION_KEY_HASH};
use crate::backend::DefaultBackend;
use crate::circuits::get_bytecode_from_program;
use crate::prove::prove;
use crate::traits::{Prove, Verify};
use crate::util::write_to_temp_file;
use crate::verify::{verify, VerificationKey, VerificationKeyHash};
use crate::Result;
use core::iter::Iterator;
use element::Base;
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

const PROGRAM: &str = include_str!("../../../../fixtures/programs/agg_utxo.json");
const KEY: &[u8] = include_bytes!("../../../../fixtures/keys/agg_utxo_key");
const KEY_FIELDS: &[u8] = include_bytes!("../../../../fixtures/keys/agg_utxo_key_fields.json");

lazy_static! {
    static ref PROGRAM_ARTIFACT: ProgramArtifact = serde_json::from_str(PROGRAM).unwrap();
    static ref PROGRAM_COMPILED: CompiledProgram = CompiledProgram::from(PROGRAM_ARTIFACT.clone());
    static ref PROGRAM_PATH: PathBuf = write_to_temp_file(PROGRAM.as_bytes(), ".json");
    static ref BYTECODE: Vec<u8> = get_bytecode_from_program(PROGRAM);
    pub static ref AGG_UTXO_VERIFICATION_KEY: VerificationKey =
        VerificationKey(serde_json::from_slice(KEY_FIELDS).unwrap());
    pub static ref AGG_UTXO_VERIFICATION_KEY_HASH: VerificationKeyHash = VerificationKeyHash(
        bn254_blackbox_solver::poseidon_hash(&AGG_UTXO_VERIFICATION_KEY.0, false).unwrap()
    );
}

const AGG_UTXO_PUBLIC_INPUTS_COUNT: usize = 21;

impl Prove for AggUtxo {
    type Proof = AggUtxoProof;
    type Result<Proof> = Result<Proof>;

    fn prove(&self) -> Self::Result<Self::Proof> {
        println!("=====================");
        println!("Proving AGG_UTXO...");
        println!("=====================");

        let agg_input = AggUtxoInput::from(self);
        // println!(
        //     "public inputs: {:?}",
        //     &self.proofs[0].utxo_proof.public_inputs
        // );

        let inputs = InputMap::from(agg_input);

        // println!("inputs: {:?}", inputs);

        // println!(
        //     "AGG_UTXO_VERIFICATION_KEY_HASH: {}",
        //     Element::from_base(AGG_UTXO_VERIFICATION_KEY_HASH.0).to_hex()
        // );

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
        let public_inputs = proof_bytes[..AGG_UTXO_PUBLIC_INPUTS_COUNT * 32].to_vec();
        let public_inputs = bytes_to_elements(&public_inputs);
        let raw_proof = proof_bytes[AGG_UTXO_PUBLIC_INPUTS_COUNT * 32..].to_vec();

        assert_eq!(
            public_inputs.len(),
            AGG_UTXO_PUBLIC_INPUTS_COUNT,
            "Public inputs must be {} elements",
            AGG_UTXO_PUBLIC_INPUTS_COUNT
        );

        assert_eq!(
            raw_proof.len(),
            507 * 32,
            "Proof must be 93 elements of 32 bytes"
        );

        Ok(AggUtxoProof {
            proof: AggUtxoProofBytes(raw_proof),
            public_inputs: AggUtxoPublicInput {
                messages: [
                    public_inputs[0],
                    public_inputs[1],
                    public_inputs[2],
                    public_inputs[3],
                    public_inputs[4],
                    public_inputs[5],
                    public_inputs[6],
                    public_inputs[7],
                    public_inputs[8],
                    public_inputs[9],
                    public_inputs[10],
                    public_inputs[11],
                    public_inputs[12],
                    public_inputs[13],
                    public_inputs[14],
                    public_inputs[15],
                    public_inputs[16],
                    public_inputs[17],
                ],
                old_root: public_inputs[18],
                new_root: public_inputs[19],
                commit_hash: public_inputs[20],
            },
        })
    }
}

impl Verify for AggUtxoProof {
    fn verify(&self) -> Result<()> {
        verify::<DefaultBackend>(KEY, &self.to_bytes(), false)
    }
}

#[derive(Debug, Clone)]
pub struct AggUtxoInput {
    pub proofs: [AggUtxoProofInput; 3],
    pub messages: [Base; 18],
    pub old_root: Base,
    pub new_root: Base,
    pub commit_hash: Base,
}

impl From<&AggUtxo> for AggUtxoInput {
    fn from(agg_utxo: &AggUtxo) -> Self {
        let proofs: Vec<AggUtxoProofInput> = agg_utxo
            .proofs
            .iter()
            .map(AggUtxoProofInput::from)
            .collect();
        let messages: Vec<Base> = agg_utxo
            .proofs
            .iter()
            .flat_map(|p| p.utxo_proof.public_inputs.messages.iter())
            .map(|m| m.to_base())
            .collect();
        AggUtxoInput {
            proofs: proofs.try_into().unwrap(),
            messages: messages.try_into().unwrap(),
            old_root: agg_utxo.old_root.to_base(),
            new_root: agg_utxo.new_root.to_base(),
            commit_hash: agg_utxo.commit_hash().to_base(),
        }
    }
}

impl From<AggUtxoInput> for InputMap {
    fn from(value: AggUtxoInput) -> Self {
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
        map.insert(
            "verification_key_hash".to_owned(),
            InputValue::Field(UTXO_VERIFICATION_KEY_HASH.0),
        );

        map.insert(
            "proofs".to_owned(),
            InputValue::Vec(value.proofs.map(InputValue::from).to_vec()),
        );
        map.insert(
            "messages".to_owned(),
            InputValue::Vec(value.messages.map(InputValue::Field).to_vec()),
        );
        map.insert("old_root".to_owned(), InputValue::Field(value.old_root));
        map.insert("new_root".to_owned(), InputValue::Field(value.new_root));
        map.insert(
            "commit_hash".to_owned(),
            InputValue::Field(value.commit_hash),
        );

        map
    }
}

#[derive(Debug, Clone)]
pub struct AggUtxoProofInput {
    pub proof: [Base; 507],
    pub input_merkle_paths: [[Base; 160]; 2],
    pub output_merkle_paths: [[Base; 160]; 2],
    pub input_commitments: [Base; 2],
    pub output_commitments: [Base; 2],
}

impl From<&UtxoProofBundleWithMerkleProofs> for AggUtxoProofInput {
    fn from(value: &UtxoProofBundleWithMerkleProofs) -> Self {
        let convert_merkle_paths = |merkle_paths: &[MerklePath<161>; 2]| -> [[Base; 160]; 2] {
            let mut paths = [[Base::default(); 160]; 2];
            for (i, mp) in merkle_paths.iter().enumerate() {
                for (j, s) in mp.siblings.iter().enumerate() {
                    paths[i][j] = s.to_base();
                }
            }
            paths
        };

        AggUtxoProofInput {
            proof: value
                .utxo_proof
                .proof
                .to_fields()
                .iter()
                .map(|e| e.to_base())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            input_merkle_paths: convert_merkle_paths(&value.input_merkle_paths),
            output_merkle_paths: convert_merkle_paths(&value.output_merkle_paths),
            input_commitments: value
                .utxo_proof
                .public_inputs
                .input_commitments
                .map(|e| e.to_base()),
            output_commitments: value
                .utxo_proof
                .public_inputs
                .output_commitments
                .map(|e| e.to_base()),
        }
    }
}

impl From<AggUtxoProofInput> for InputValue {
    fn from(value: AggUtxoProofInput) -> Self {
        let mut struct_ = BTreeMap::new();
        struct_.insert(
            "proof".to_owned(),
            InputValue::Vec(value.proof.map(InputValue::Field).to_vec()),
        );
        struct_.insert(
            "input_merkle_paths".to_owned(),
            InputValue::Vec(
                value
                    .input_merkle_paths
                    .map(|mp| InputValue::Vec(mp.map(InputValue::Field).to_vec()))
                    .to_vec(),
            ),
        );

        struct_.insert(
            "output_merkle_paths".to_owned(),
            InputValue::Vec(
                value
                    .output_merkle_paths
                    .map(|mp| InputValue::Vec(mp.map(InputValue::Field).to_vec()))
                    .to_vec(),
            ),
        );

        struct_.insert(
            "input_commitments".to_owned(),
            InputValue::Vec(value.input_commitments.map(InputValue::Field).to_vec()),
        );

        struct_.insert(
            "output_commitments".to_owned(),
            InputValue::Vec(value.output_commitments.map(InputValue::Field).to_vec()),
        );

        InputValue::Struct(struct_)
    }
}
