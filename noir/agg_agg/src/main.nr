use dep::std;
use poseidon::poseidon2;

struct AggProof {
    proof: [Field; 508],
    old_root: Field,
    new_root: Field,
    commit_hash: Field,
    messages: [Field; 1000],
    verification_key: [Field; 115],
    verification_key_hash: Field
}

fn main(
    proofs: [AggProof; 2],
    // allowed proofs (will be validated by agg_final)
    verification_key_hash: pub [Field; 2],
    old_root: pub Field,
    new_root: pub Field,
    commit_hash: pub Field,
    messages: pub [Field; 1000],
) {
    let mut root = old_root;
    let mut first_public_inputs: [Field; 1005] = [0; 1005];
    let mut messages_index = 0;

    for i in 0..2 {
        let proof = proofs[i];

        // Assert the proof is one of public verification_key_hash (this will be verified in the agg_final)
        let is_valid_proof = (verification_key_hash[0] == proof.verification_key_hash) | (verification_key_hash[1] == proof.verification_key_hash);
        assert(is_valid_proof, "proof must be one of the valid proofs");

        let mut public_inputs: [Field; 1005] = [0; 1005];

        public_inputs[0] = verification_key_hash[0];
        public_inputs[1] = verification_key_hash[1];
        public_inputs[2] = proof.old_root;
        public_inputs[3] = proof.new_root;
        public_inputs[4] = proof.commit_hash;

        ///////
        // Check compact messages and assign public inputs
        ///////

        // End immediately for padding proof (as it cannot have messages)
        let mut has_ended = proof.old_root == 0;

        // Exportable message kinds (2, 3 and 4) consume the next x messages, so when checking
        // for the end we skip the consumed messages, so we can find the first non-exportable kind.
        // Assumes that agg_utxo proof also compacts exportable kinds from index 0 (without gaps).
        let mut next_check = 0;
        for j in 0..1000 {
            let proof_message = proof.messages[j];
            public_inputs[j + 5] = proof_message;

            // Update next checkpoint (or end)
            if (next_check == j) {
                if proof_message == 2 {
                    // Mint
                    next_check += 4;
                } else if ((proof_message == 3) | (proof_message == 4)) {
                    // Burn, Swap
                    next_check += 5;
                } else {
                    has_ended = true;
                }
            }

            if (has_ended) {
                assert(proof_message == 0, "messages cannot have gaps");
            } else {
                assert(messages_index != 1000, "proof messages exceed compacted range");
                assert(proof_message == messages[messages_index], "proof message must be exported");
                // Incr the global counter (shared between proofs)
                messages_index += 1;
            }
        }

        // Workaround for 2nd proof being a padding proof
        if i == 0 {
            first_public_inputs = public_inputs;
        }

        std::verify_proof_with_type(
            proof.verification_key,
            if proof.old_root != 0 {
                proof.proof
            } else {
                proofs[0].proof
            },
            if proof.old_root != 0 {
                public_inputs
            } else {
                first_public_inputs
            },
            proof.verification_key_hash,
            7,
        );

        if proof.old_root != 0 {
            assert(proof.old_root == root, "proof old_root must match previous root");
            root = proof.new_root;
        }
    }

    assert(
        commit_hash
            == poseidon2::Poseidon2::hash(
                [proofs[0].commit_hash, proofs[1].commit_hash],
                2,
            ),
    );
    assert(new_root == root, "Roots must match");
}
