use super::{AGG_UTXO_VERIFICATION_KEY, AGG_UTXO_VERIFICATION_KEY_HASH};
use crate::backend::DefaultBackend;
use crate::circuits::get_bytecode_from_program;
use crate::util::write_to_temp_file;
use crate::verify::{verify, VerificationKey, VerificationKeyHash};
use crate::Result;
use crate::{
    prove::prove,
    traits::{Prove, Verify},
};
use element::Base;
use lazy_static::lazy_static;
use noirc_abi::{input_parser::InputValue, InputMap};
use noirc_artifacts::program::ProgramArtifact;
use noirc_driver::CompiledProgram;
use std::collections::BTreeMap;
use std::path::PathBuf;
use zk_primitives::{
    bytes_to_elements, AggAgg, AggAggProof, AggAggProofBytes, AggAggPublicInput, AggUtxoProof,
    ToBytes,
};

const PROGRAM: &str = include_str!("../../../../fixtures/programs/agg_agg.json");
const KEY: &[u8] = include_bytes!("../../../../fixtures/keys/agg_agg_key");
const KEY_FIELDS: &[u8] = include_bytes!("../../../../fixtures/keys/agg_agg_key_fields.json");

lazy_static! {
    static ref PROGRAM_ARTIFACT: ProgramArtifact = serde_json::from_str(PROGRAM).unwrap();
    static ref PROGRAM_COMPILED: CompiledProgram = CompiledProgram::from(PROGRAM_ARTIFACT.clone());
    static ref PROGRAM_PATH: PathBuf = write_to_temp_file(PROGRAM.as_bytes(), ".json");
    static ref BYTECODE: Vec<u8> = get_bytecode_from_program(PROGRAM);
    pub static ref AGG_AGG_VERIFICATION_KEY: VerificationKey =
        VerificationKey(serde_json::from_slice(KEY_FIELDS).unwrap());
    pub static ref AGG_AGG_VERIFICATION_KEY_HASH: VerificationKeyHash = VerificationKeyHash(
        bn254_blackbox_solver::poseidon_hash(&AGG_AGG_VERIFICATION_KEY.0, false).unwrap()
    );
}

// AggAgg public input fields:
// - verification_key_hash: Base (1 field)
// - old_root: Base (1 field)
// - new_root: Base (1 field)
// - commit_hash: Base (1 field)
// - messages: Vec<Base> (36 fields => 2 proofs * 18 messages each)
// - kzg: Vec<Base> (16 fields)
// Total: 56 fields
const AGG_AGG_PUBLIC_INPUTS_COUNT: usize = 1 + 1 + 1 + 1 + 36 + 16;

impl Prove for AggAgg {
    type Proof = AggAggProof;
    type Result<Proof> = Result<Proof>;

    fn prove(&self) -> Self::Result<Self::Proof> {
        let inputs = InputMap::from(AggAggInput::from(self));

        let proof_bytes = prove::<DefaultBackend>(
            &PROGRAM_COMPILED,
            PROGRAM.as_bytes(),
            &BYTECODE,
            &inputs,
            false,
            true,
        )?;
        let public_inputs_bytes = proof_bytes[..AGG_AGG_PUBLIC_INPUTS_COUNT * 32].to_vec();
        let public_inputs = bytes_to_elements(&public_inputs_bytes);
        let raw_proof = proof_bytes[AGG_AGG_PUBLIC_INPUTS_COUNT * 32..].to_vec();

        assert_eq!(
            public_inputs.len(),
            AGG_AGG_PUBLIC_INPUTS_COUNT,
            "Public inputs must be {} elements",
            AGG_AGG_PUBLIC_INPUTS_COUNT
        );
        assert_eq!(
            raw_proof.len(),
            440 * 32,
            "Proof must be 440 elements of 32 bytes"
        );

        let p = AggAggProof {
            proof: AggAggProofBytes(raw_proof),
            public_inputs: AggAggPublicInput {
                verification_key_hash: public_inputs[0],
                old_root: public_inputs[1],
                new_root: public_inputs[2],
                commit_hash: public_inputs[3],
                messages: public_inputs[4..4 + 6 * 6].to_vec(),
            },
            kzg: public_inputs[public_inputs.len() - 16..].to_vec(),
        };
        Ok(p)
    }
}

impl Verify for AggAggProof {
    fn verify(&self) -> Result<()> {
        verify::<DefaultBackend>(KEY, &self.to_bytes(), true)
    }
}

#[derive(Debug, Clone)]
pub struct AggAggInput {
    pub proofs: [UtxoAggProof; 2],
    pub messages: [Base; 2 * 18],
    pub old_root: Base,
    pub new_root: Base,
    pub commit_hash: Base,
}

impl From<&AggAgg> for AggAggInput {
    fn from(agg_agg: &AggAgg) -> Self {
        AggAggInput {
            proofs: [
                UtxoAggProof::from(&agg_agg.proofs[0]),
                UtxoAggProof::from(&agg_agg.proofs[1]),
            ],
            messages: extract_messages(&agg_agg.proofs),
            old_root: agg_agg.old_root().to_base(),
            new_root: agg_agg.new_root().to_base(),
            commit_hash: agg_agg.commit_hash().to_base(),
        }
    }
}

fn extract_messages(agg_agg_proofs: &[AggUtxoProof; 2]) -> [Base; 2 * 18] {
    agg_agg_proofs
        .iter()
        .flat_map(|proof| {
            proof
                .public_inputs
                .messages
                .iter()
                .map(|message| message.to_base())
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

impl From<AggAggInput> for InputMap {
    fn from(value: AggAggInput) -> Self {
        let mut map = InputMap::new();

        // Should be static
        map.insert(
            "verification_key".to_owned(),
            InputValue::Vec(
                AGG_UTXO_VERIFICATION_KEY
                    .0
                    .iter()
                    .cloned()
                    .map(InputValue::Field)
                    .collect(),
            ),
        );
        map.insert(
            "verification_key_hash".to_owned(),
            InputValue::Field(AGG_UTXO_VERIFICATION_KEY_HASH.0),
        );

        map.insert(
            "utxo_agg_proofs".to_owned(),
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
pub struct UtxoAggProof {
    pub proof: [Base; 456],
    pub old_root: Base,
    pub new_root: Base,
    pub commit_hash: Base,
}

impl From<&AggUtxoProof> for UtxoAggProof {
    fn from(value: &AggUtxoProof) -> Self {
        UtxoAggProof {
            proof: value
                .proof
                .to_fields()
                .iter()
                .map(|e| e.to_base())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            old_root: value.public_inputs.old_root.to_base(),
            new_root: value.public_inputs.new_root.to_base(),
            commit_hash: value.public_inputs.commit_hash.to_base(),
        }
    }
}

impl From<UtxoAggProof> for InputValue {
    fn from(value: UtxoAggProof) -> Self {
        let mut struct_ = BTreeMap::new();

        struct_.insert(
            "proof".to_owned(),
            InputValue::Vec(value.proof.map(InputValue::Field).to_vec()),
        );
        struct_.insert("old_root".to_owned(), InputValue::Field(value.old_root));
        struct_.insert("new_root".to_owned(), InputValue::Field(value.new_root));
        struct_.insert(
            "commit_hash".to_owned(),
            InputValue::Field(value.commit_hash),
        );

        InputValue::Struct(struct_)
    }
}

// #[derive(Debug, Clone)]
// pub struct Input {
//     pub verification_key: [Base; 114],
//     pub proofs: [AggUtxoProof; 2],
//     pub kinds: [Base; 6],
//     pub old_root: Base,
//     pub new_root: Base,
//     pub key_hash: Base,
// }

// #[derive(Debug, Clone)]
// pub struct AggUtxoProof {
//     pub proof: [Base; 109],
//     pub old_root: Base,
//     pub new_root: Base,
// }

// impl From<Input> for InputMap {
//     fn from(value: Input) -> Self {
//         let mut map = InputMap::new();

//         map.insert(
//             "verification_key".to_owned(),
//             InputValue::Vec(value.verification_key.map(InputValue::Field).to_vec()),
//         );
//         map.insert(
//             "utxo_agg_proofs".to_owned(),
//             InputValue::Vec(value.proofs.map(InputValue::from).to_vec()),
//         );
//         map.insert(
//             "kinds".to_owned(),
//             InputValue::Vec(value.kinds.map(InputValue::Field).to_vec()),
//         );
//         map.insert("old_root".to_owned(), InputValue::Field(value.old_root));
//         map.insert("new_root".to_owned(), InputValue::Field(value.new_root));
//         map.insert("key_hash".to_owned(), InputValue::Field(value.key_hash));

//         map
//     }
// }

// impl From<AggUtxoProof> for InputValue {
//     fn from(value: AggUtxoProof) -> Self {
//         let mut struct_ = BTreeMap::new();

//         struct_.insert(
//             "proof".to_owned(),
//             InputValue::Vec(value.proof.map(InputValue::Field).to_vec()),
//         );
//         struct_.insert("old_root".to_owned(), InputValue::Field(value.old_root));
//         struct_.insert("new_root".to_owned(), InputValue::Field(value.new_root));

//         InputValue::Struct(struct_)
//     }
// }

// #[derive(Default, Debug, Clone)]
// struct BorshBase(pub Base);

// impl BorshSerialize for BorshBase {
//     fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
//         writer.write_all(&self.0.to_be_bytes())
//     }
// }

// impl BorshDeserialize for BorshBase {
//     fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
//         let mut bytes = [0u8; 32];
//         reader.read_exact(&mut bytes)?;

//         Ok(BorshBase(Base::from_be_bytes_reduce(&bytes)))
//     }
// }

// impl Deref for BorshBase {
//     type Target = Base;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl DerefMut for BorshBase {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

// impl From<Base> for BorshBase {
//     fn from(value: Base) -> Self {
//         BorshBase(value)
//     }
// }

// #[derive(Default, Debug, Clone, BorshSerialize, BorshDeserialize)]
// pub struct PublicInput {
//     pub kinds: [BorshBase; 6],
//     pub old_root: BorshBase,
//     pub new_root: BorshBase,
// }

// impl Barretenberg {
//     pub fn agg_agg_prove(&self, input: Input) -> crate::Result<Vec<u8>> {
//         let results = crate::execute::execute_program_and_decode(
//             self.program.clone().into(),
//             &InputMap::from(input),
//             false,
//         )
//         .unwrap();
//         let witness_gz = TryInto::<Vec<u8>>::try_into(results.witness_stack).unwrap();

//         let proof = self.prove(&witness_gz, true)?;

//         Ok(proof)
//     }

//     pub fn agg_agg_verify(&self, proof: &[u8]) -> crate::Result<bool> {
//         self.verify(KEY, proof)
//     }

//     pub fn agg_agg_proof_fields(&self, proof: &[u8]) -> crate::Result<PublicInput> {
//         let fields = self.proof_as_fields(KEY, proof)?;

//         Ok(PublicInput {
//             kinds: [
//                 fields[0], fields[1], fields[2], fields[3], fields[4], fields[5],
//             ]
//             .map(Into::into),
//             old_root: fields[6].into(),
//             new_root: fields[7].into(),
//         })
//     }
// }

// impl From<Input> for PublicInput {
//     fn from(value: Input) -> Self {
//         Self {
//             kinds: value.kinds.map(Into::into),
//             old_root: value.old_root.into(),
//             new_root: value.new_root.into(),
//         }
//     }
// }
