use dep::std;
use poseidon::poseidon2;

mod test;
pub mod test_fixtures;

struct AggUtxoProofInput {
    proof: [Field; 507],
    input_merkle_paths: [[Field; 160]; 2],
    output_merkle_paths: [[Field; 160]; 2],
    input_commitments: [Field; 2],
    output_commitments: [Field; 2],
}

fn main(
    verification_key: [Field; 112],
    verification_key_hash: Field,
    proofs: [AggUtxoProofInput; 3],
    messages: pub [Field; 18],
    old_root: pub Field,
    new_root: pub Field,
    commit_hash: pub Field
) {
    // Hardcode verification_key_hash for utxo proof
    assert(verification_key_hash == 4180082733336422412042586419047349326944586166886980312516770758826360875574, "only utxo proof allowed");

    let mut root = old_root;
    let mut utxo_hashes: [Field; 3] = [0,0,0];

    for i in 0..3 {
        let proof = proofs[i];
        let j = i * 6;
        let kind = messages[j];
        let input_commitments = proof.input_commitments;
        let output_commitments = proof.output_commitments;

        utxo_hashes[i] = poseidon2::Poseidon2::hash([input_commitments[0], input_commitments[1], output_commitments[0], output_commitments[1]], 4);

        println(kind);

        // Verify non-padding proofs
        if (kind != 0) { // this DOES NOT work
        // if (i == 0) { // this DOES work (but not the logic we need)
            let public_inputs = [
                input_commitments[0],
                input_commitments[1],
                output_commitments[0],
                output_commitments[1],
                messages[j],
                messages[j + 1],
                messages[j + 2],
                messages[j + 3],
                messages[j + 4],
                messages[j + 5],
            ];
            std::verify_proof_with_type(
                verification_key,
                proof.proof,
                public_inputs,
                0x0,
                7,
            );

            // Input commitments
            for k in 0..2 {
                let commitment = input_commitments[k];

                if commitment != 0 {
                    let bits: [u1; 254] = commitment.to_le_bits();
                    let merkle_path = proof.input_merkle_paths[k];

                    // Check it exists in the old tree
                    validate_inclusion(commitment, bits, merkle_path, root);

                    // Calculate new root without the input commitment
                    root = get_null_root(bits, merkle_path);
                }
            }

            // Output commitments
            for k in 0..2 {
                let commitment = output_commitments[k];

                if commitment != 0 {
                    let bits: [u1; 254] = commitment.to_le_bits();
                    let merkle_path = proof.output_merkle_paths[k];

                    // Calculate new root with added output commitments
                    root = get_merkle_root(commitment, bits, merkle_path);
                }
            }
        }
    }

    assert(root == new_root, "New root is not valid");
    assert(commit_hash == poseidon2::Poseidon2::hash(utxo_hashes, 3));
}

fn validate_inclusion(leaf: Field, bits: [u1; 254], merkle_path: [Field; 160], root: Field) {
    let merkle_root = get_merkle_root(leaf, bits, merkle_path);
    assert(merkle_root == root, "Merkle path root does not match");
}

fn get_null_root(bits: [u1; 254], merkle_path: [Field; 160]) -> Field {
    get_merkle_root(0, bits, merkle_path)
}

fn get_merkle_root(leaf: Field, bits: [u1; 254], merkle_path: [Field; 160]) -> Field {
    let mut hash = leaf;
    for i in 0..160 {
        let dir = bits[i];
        let sibling = merkle_path[i];
        if dir == 0 {
            hash = poseidon2::Poseidon2::hash([hash, sibling], 2);
        } else {
            hash = poseidon2::Poseidon2::hash([sibling, hash], 2);
        }
    }
    hash
}
