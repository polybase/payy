use dep::std;
use poseidon::poseidon2;

global UTXO_VERIFICATION_KEY_HASH: Field = 20361994226032552507663901262286744551869524558436762371297713593361795171294;

struct AggUtxoProofInput {
    proof: [Field; 508],
    utxo_kind: Field,
    input_merkle_paths: [[Field; 160]; 2],
    output_merkle_paths: [[Field; 160]; 2],
    input_commitments: [Field; 2],
    output_commitments: [Field; 2],
}

fn main(
    verification_key: [Field; 115],
    proofs: [AggUtxoProofInput; 3],
    // TODO: maybe we can remove this
    verification_key_hash: pub [Field; 2],
    old_root: pub Field,
    new_root: pub Field,
    commit_hash: pub Field,
    messages: pub [Field; 1000],
) {
    // Hardcode verification_key_hash for utxo proof
    assert(verification_key_hash[0] != 0, "dummy verfication key hash");
    assert(verification_key_hash[1] != 0, "dummy verfication key hash");

    let mut root = old_root;
    let mut utxo_hashes: [Field; 3] = [0, 0, 0];
    let mut messages_index = 0;

    let mut first_public_inputs: [Field; 9] = [0; 9];
    for i in 0..3 {
        let proof = proofs[i];
        let input_commitments = proof.input_commitments;
        let output_commitments = proof.output_commitments;

        let utxo_kind = proof.utxo_kind;
        let is_padding = utxo_kind == 0;
        let is_burn = utxo_kind == 3;
        let is_mint = utxo_kind == 2;
        let is_proof_kind_mint_or_burn = is_burn | is_mint;

        let mut msg_note_kind = 0;
        let mut msg_value = 0;
        let mut msg_hash = 0;
        let mut msg_burn_addr = 0;

        if is_proof_kind_mint_or_burn {
            assert(messages[messages_index] == utxo_kind, "proof 'kind' must match message");
            msg_note_kind = messages[messages_index + 1];
            msg_value = messages[messages_index + 2];
            msg_hash = messages[messages_index + 3];
            messages_index += 4;
        }

        if is_burn {
            msg_burn_addr = messages[messages_index];
            messages_index += 1;
        }

        utxo_hashes[i] = poseidon2::Poseidon2::hash(
            [
                input_commitments[0],
                input_commitments[1],
                output_commitments[0],
                output_commitments[1],
            ],
            4,
        );

        // Verify non-padding proofs
        let public_inputs = [
            input_commitments[0],
            input_commitments[1],
            output_commitments[0],
            output_commitments[1],
            utxo_kind,
            msg_note_kind,
            msg_value,
            msg_hash,
            msg_burn_addr,
        ];

        // Capture the public_inputs on first iter
        if (i == 0) {
            // First proof must ALWAYS be a real proof
            assert(!is_padding, "first proof cannot be a padding proof");
            // and we will use that proof
            first_public_inputs = public_inputs;
        }

        // Due to a limitation in noir, verify_proof_with_type will ALWAYS
        // be run regardless of whether its inside an if statement, so we need
        // to form a valid proof. As first proof cannot be a dummy proof, we can
        // use the first proof for padding elements
        std::verify_proof_with_type(
            verification_key,
            if (is_padding) {
                proofs[0].proof
            } else {
                proof.proof
            },
            if (is_padding) {
                first_public_inputs
            } else {
                public_inputs
            },
            UTXO_VERIFICATION_KEY_HASH,
            7,
        );

        if (!is_padding) {
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

                    // Checks the merkle path leads to the existing root
                    validate_null(bits, merkle_path, root);

                    // Calculate new root with added output commitments
                    root = get_merkle_root(commitment, bits, merkle_path);
                }
            }
        }
    }

    // Verify all other messages are zero
    for i in 0..messages.len() {
        if i >= messages_index {
            assert(messages[i] == 0, "Message is not zero");
        }
    }

    assert(root == new_root, "New root is not valid");
    assert(commit_hash == poseidon2::Poseidon2::hash(utxo_hashes, 3));
}

fn validate_inclusion(leaf: Field, bits: [u1; 254], merkle_path: [Field; 160], root: Field) {
    let merkle_root = get_merkle_root(leaf, bits, merkle_path);
    assert(merkle_root == root, "Merkle path root does not match");
}

fn validate_null(bits: [u1; 254], merkle_path: [Field; 160], root: Field) {
    let merkle_root = get_null_root(bits, merkle_path);
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
