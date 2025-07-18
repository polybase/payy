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
use noirc_abi::InputMap;
use noirc_artifacts::program::ProgramArtifact;
use noirc_driver::CompiledProgram;
use std::path::PathBuf;
use zk_primitives::{
    bytes_to_elements, get_address_for_private_key, Signature, SignatureProof, SignatureProofBytes,
    SignaturePublicInput, ToBytes,
};

const PROGRAM: &str = include_str!("../../../../fixtures/programs/signature.json");
const KEY: &[u8] = include_bytes!("../../../../fixtures/keys/signature_key");

lazy_static! {
    static ref PROGRAM_ARTIFACT: ProgramArtifact = serde_json::from_str(PROGRAM).unwrap();
    static ref PROGRAM_COMPILED: CompiledProgram = CompiledProgram::from(PROGRAM_ARTIFACT.clone());
    static ref PROGRAM_PATH: PathBuf = write_to_temp_file(PROGRAM.as_bytes(), ".json");
    static ref BYTECODE: Vec<u8> = get_bytecode_from_program(PROGRAM);
}

// TODO: can we move this as a trait on zk-primitives?
const SIGNATURE_PUBLIC_INPUTS_COUNT: usize = 2;
const SIGNATURE_PROOF_SIZE: usize = 440;

impl Prove for Signature {
    type Proof = SignatureProof;
    type Result<Proof> = Result<Proof>;

    fn prove(&self) -> Self::Result<Self::Proof> {
        let inputs = InputMap::from(SignatureInput::from(self));

        let proof_bytes = prove::<DefaultBackend>(
            &PROGRAM_COMPILED,
            PROGRAM.as_bytes(),
            &BYTECODE,
            &inputs,
            false,
            false,
        )?;

        // Slice off the public inputs
        let public_inputs = proof_bytes[..SIGNATURE_PUBLIC_INPUTS_COUNT * 32].to_vec();
        let public_inputs = bytes_to_elements(&public_inputs);
        let raw_proof = proof_bytes[SIGNATURE_PUBLIC_INPUTS_COUNT * 32..].to_vec();

        assert_eq!(
            raw_proof.len(),
            SIGNATURE_PROOF_SIZE * 32,
            "Proof must be {SIGNATURE_PROOF_SIZE} elements of 32 bytes"
        );

        // Convert the proof bytes to a UtxoProof
        let proof = SignatureProof {
            proof: SignatureProofBytes(raw_proof),
            public_inputs: SignaturePublicInput {
                address: public_inputs[0],
                message: public_inputs[1],
            },
        };

        Ok(proof)
    }
}

impl Verify for SignatureProof {
    fn verify(&self) -> Result<()> {
        verify::<DefaultBackend>(KEY, &self.to_bytes(), false)
    }
}

#[derive(Debug, Clone)]
struct SignatureInput {
    secret_key: Base,
    message: Base,
    message_hash: Base,
    address: Base,
}

impl From<&Signature> for SignatureInput {
    fn from(signature: &Signature) -> Self {
        SignatureInput {
            secret_key: signature.secret_key.to_base(),
            message: signature.message.to_base(),
            message_hash: signature.message_hash().to_base(),
            address: get_address_for_private_key(signature.secret_key).to_base(),
        }
    }
}

impl From<SignatureInput> for InputMap {
    fn from(input: SignatureInput) -> Self {
        let mut map = InputMap::new();

        map.insert(
            "owner_pk".to_owned(),
            noirc_abi::input_parser::InputValue::Field(input.secret_key),
        );
        map.insert(
            "message_hash".to_owned(),
            noirc_abi::input_parser::InputValue::Field(input.message_hash),
        );
        map.insert(
            "address".to_owned(),
            noirc_abi::input_parser::InputValue::Field(input.address),
        );
        map.insert(
            "message".to_owned(),
            noirc_abi::input_parser::InputValue::Field(input.message),
        );

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use element::Element;

    #[test]
    fn test_signature_proof_generation_and_verification() {
        // Create a signature with a test private key and message
        let secret_key = Element::from(101u64);
        let message = Element::from(100u64);
        let signature = Signature {
            secret_key,
            message,
        };

        // Generate a proof for the signature
        let proof = signature.prove().unwrap();

        // Verify the proof
        proof.verify().expect("Verification failed");
    }

    #[test]
    fn test_signature_input_conversion() {
        // Create a signature
        let secret_key = Element::from(101u64);
        let message = Element::from(101u64);
        let signature = Signature {
            secret_key,
            message,
        };

        // Convert to SignatureInput
        let input = SignatureInput::from(&signature);

        // Verify the conversion
        assert_eq!(input.secret_key, secret_key.to_base());
        assert_eq!(input.message, message.to_base());
        assert_eq!(
            input.address,
            get_address_for_private_key(secret_key).to_base()
        );
    }
}
