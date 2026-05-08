use payy_evm_client_prover_interface::{Error, PrivacyCircuit, ProofDecodeErrorKind, Result};
use zk_circuits::circuits::{
    Proof, ProofDecodeError, ProofDecodeErrorKind as CircuitProofDecodeErrorKind, PublicInputs,
};

pub fn decode_proof<P: PublicInputs>(circuit: PrivacyCircuit, proof: Vec<u8>) -> Result<Proof<P>> {
    Proof::<P>::try_from_raw_proof_bytes(proof).map_err(|err| Error::InvalidProof {
        circuit,
        kind: proof_decode_error_kind(err),
    })
}

fn proof_decode_error_kind(err: ProofDecodeError) -> ProofDecodeErrorKind {
    match err.kind() {
        CircuitProofDecodeErrorKind::PublicInputLengthOverflow => {
            ProofDecodeErrorKind::PublicInputLengthOverflow
        }
        CircuitProofDecodeErrorKind::PublicInputsTooShort => {
            ProofDecodeErrorKind::PublicInputsTooShort
        }
        CircuitProofDecodeErrorKind::LengthNotMultipleOfField => {
            ProofDecodeErrorKind::LengthNotMultipleOfField
        }
    }
}
