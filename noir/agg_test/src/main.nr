use dep::std;
use std::hash::poseidon2;

fn main(
    verification_key: [Field; 112],
    // verification_key_hash: Field,
    proof: [Field; 507],
    public_inputs: [Field; 10],
) {
    // Hardcode verification_key_hash for utxo proof
    // assert(verification_key_hash == 4180082733336422412042586419047349326944586166886980312516770758826360875574, "only utxo proof allowed");

    std::verify_proof_with_type(
        verification_key,
        proof,
        public_inputs,
        0x0,
        7,
    );
}
