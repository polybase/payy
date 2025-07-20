use poseidon::poseidon2;
use super::{main, AggUtxoProofInput, validate_inclusion, get_null_root, get_merkle_root};
use super::test_fixtures::{P1, P2, P3, P4, COMMITMENTS, OLD_ROOT, NEW_ROOT, UTXO_VERIFICATION_KEY};

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
    let verification_key = [0; 112];
    let invalid_hash = 1234567890; // Wrong hash

    let proofs = [
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        },
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        },
        AggUtxoProofInput {
            proof: [0; 507],
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

#[test]
fn test_one_proof_with_two_padding() {
    let verification_key = [0; 112];
    let verification_key_hash = UTXO_VERIFICATION_KEY;

    let commitments_hash = poseidon2::Poseidon2::hash(COMMITMENTS, 4);

    let proofs = [
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [P1, P2],
            output_merkle_paths: [P3, P4],
            input_commitments: [COMMITMENTS[0], COMMITMENTS[1]],
            output_commitments: [COMMITMENTS[2], COMMITMENTS[3]],
        },
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        },
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        }
    ];

    let messages = [
        1, 0, 0, 0, 0, 0,  // First proof: kind = 1 (non-padding)
        0, 0, 0, 0, 0, 0,  // Second proof: kind = 0 (padding)
        0, 0, 0, 0, 0, 0   // Third proof: kind = 0 (padding)
    ];
    let old_root = OLD_ROOT;
    let new_root = NEW_ROOT;

    let zero_commit_hash = poseidon2::Poseidon2::hash([0, 0, 0, 0], 4);
    let commit_hash = poseidon2::Poseidon2::hash([commitments_hash, zero_commit_hash, zero_commit_hash], 3);

    main(verification_key, verification_key_hash, proofs, messages, old_root, new_root, commit_hash);
}

#[test]
fn test_utxo_padding_notes() {
    let verification_key = [0; 112];
    let verification_key_hash = UTXO_VERIFICATION_KEY;

    let commitments_hash = poseidon2::Poseidon2::hash(COMMITMENTS, 4);

    let proofs = [
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [P1, P2],
            output_merkle_paths: [P3, P4],
            input_commitments: [COMMITMENTS[0], COMMITMENTS[1]],
            output_commitments: [COMMITMENTS[2], COMMITMENTS[3]],
        },
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        },
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        }
    ];

    let messages = [
        1, 0, 0, 0, 0, 0,  // First proof: kind = 1 (non-padding)
        1, 0, 0, 0, 0, 0,  // Second proof: kind = 0 (send, all padding commitments)
        0, 0, 0, 0, 0, 0   // Third proof: kind = 0 (padding)
    ];
    let old_root = OLD_ROOT;
    let new_root = NEW_ROOT;

    let zero_commit_hash = poseidon2::Poseidon2::hash([0, 0, 0, 0], 4);
    let commit_hash = poseidon2::Poseidon2::hash([commitments_hash, zero_commit_hash, zero_commit_hash], 3);

    main(verification_key, verification_key_hash, proofs, messages, old_root, new_root, commit_hash);
}

#[test(should_fail_with = "Merkle path root does not match")]
fn test_invalid_merkle_oath() {
    let verification_key = [0; 112];
    let verification_key_hash = UTXO_VERIFICATION_KEY;

    let commitments_hash = poseidon2::Poseidon2::hash(COMMITMENTS, 4);

    let proofs = [
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [P1, P2],
            output_merkle_paths: [P3, P4],
            input_commitments: [COMMITMENTS[0], COMMITMENTS[1]],
            output_commitments: [COMMITMENTS[2], COMMITMENTS[3]],
        },
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [1; 2],
            output_commitments: [1; 2],
        },
        AggUtxoProofInput {
            proof: [0; 507],
            input_merkle_paths: [[0; 160]; 2],
            output_merkle_paths: [[0; 160]; 2],
            input_commitments: [0; 2],
            output_commitments: [0; 2],
        }
    ];

    let messages = [
        1, 0, 0, 0, 0, 0,  // First proof: kind = 1 (non-padding)
        1, 0, 0, 0, 0, 0,  // Second proof: kind = 0 (send, all padding commitments)
        0, 0, 0, 0, 0, 0   // Third proof: kind = 0 (padding)
    ];
    let old_root = OLD_ROOT;
    let new_root = NEW_ROOT;

    let zero_commit_hash = poseidon2::Poseidon2::hash([0, 0, 0, 0], 4);
    let commit_hash = poseidon2::Poseidon2::hash([commitments_hash, zero_commit_hash, zero_commit_hash], 3);

    main(verification_key, verification_key_hash, proofs, messages, old_root, new_root, commit_hash);
}
