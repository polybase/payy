use dep::std;
use std::hash::poseidon2;

struct AggUtxoProofInput {
    proof: [Field; 456],
    input_merkle_paths: [[Field; 160]; 2],
    output_merkle_paths: [[Field; 160]; 2],
    input_commitments: [Field; 2],
    output_commitments: [Field; 2],
}

fn main(
    verification_key: [Field; 128],
    verification_key_hash: Field,
    proofs: [AggUtxoProofInput; 3],
    messages: pub [Field; 18],
    old_root: pub Field,
    new_root: pub Field,
    commit_hash: pub Field
) {
    // Hardcode verification_key_hash for utxo proof
    assert(verification_key_hash == 6266514486265361775229966232065171160965103375136380958210310093105182950319, "only utxo proof allowed");

    let mut root = old_root;
    let mut utxo_hashes: [Field; 3] = [0,0,0];

    for i in 0..3 {
        let proof = proofs[i];
        let j = i * 6;
        let kind = messages[j];
        let input_commitments = proof.input_commitments;
        let output_commitments = proof.output_commitments;

        utxo_hashes[i] = poseidon2::Poseidon2::hash([input_commitments[0], input_commitments[1], output_commitments[0], output_commitments[1]], 4);

        // Verify non-padding proofs
        if (kind != 0) {
            let public_inputs = [
                input_commitments[0],
                input_commitments[1],
                proof.output_commitments[0],
                proof.output_commitments[1],
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
                verification_key_hash,
                1,
            );

            // Input commitments
            for j in 0..2 {
                let commitment = input_commitments[j];

                if commitment != 0 {
                    let bits: [u1; 254] = commitment.to_le_bits();
                    let merkle_path = proof.input_merkle_paths[j];

                    // Check it exists in the old tree
                    validate_inclusion(commitment, bits, merkle_path, root);

                    // Calculate new root without the input commitment
                    root = get_null_root(bits, merkle_path);
                }
            }

            // Output commitments
            for j in 0..2 {
                let commitment = output_commitments[j];

                if commitment != 0 {
                    let bits: [u1; 254] = commitment.to_le_bits();
                    let merkle_path = proof.output_merkle_paths[j];

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

#[test]
fn empty_tree() {
    let mut hash = 0;
    for _ in 0..160 {
        hash = poseidon2::Poseidon2::hash([hash, hash], 2);
    }
    println("empty root hash:");
    println(hash);
    assert(hash == 2473073130432999597457871252851154814051443713747864452328961082127445883391);
}


#[test(should_fail_with = "only utxo proof allowed")]
fn test_invalid_verification_key_hash() {
    let verification_key = [0; 128];
    let invalid_hash = 1234567890; // Wrong hash

    let proofs = [
        AggUtxoProofInput {
            proof: [0; 456],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        },
        AggUtxoProofInput {
            proof: [0; 456],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        },
        AggUtxoProofInput {
            proof: [0; 456],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        }
    ];

    // First proof has kind = 1 (non-padding), so it should verify the hash
    let messages = [
        1, 0, 0, 0, 0, 0,  // First proof: kind = 1 (send)
        0, 0, 0, 0, 0, 0,  // Second proof: kind = 0 (padding)
        0, 0, 0, 0, 0, 0   // Third proof: kind = 0 (padding)
    ];
    let old_root = 2473073130432999597457871252851154814051443713747864452328961082127445883391;
    let new_root = 2473073130432999597457871252851154814051443713747864452328961082127445883391;
    let commit_hash = poseidon2::Poseidon2::hash([0, 0, 0, 0], 4);

    // This should fail because the verification key hash is incorrect
    main(verification_key, invalid_hash, proofs, messages, old_root, new_root, commit_hash);
}

#[test]
fn test_merkle_path_operations() {
    // Test basic merkle path operations
    let leaf: Field = 12345;
    let bits: [u1; 254] = leaf.to_le_bits();
    let merkle_path = [0; 160];

    // Test get_merkle_root
    let root = get_merkle_root(leaf, bits, merkle_path);
    assert(root != 0);

    // Test get_null_root (should be different from leaf root)
    let null_root = get_null_root(bits, merkle_path);
    assert(null_root != root);

    // Test validate_inclusion
    validate_inclusion(leaf, bits, merkle_path, root);
}
