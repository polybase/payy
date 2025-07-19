use dep::std;
use poseidon::poseidon2;

struct UtxoAggProof {
    proof: [Field; 456],
    old_root: Field,
    new_root: Field,
    commit_hash: Field
}

fn main(
    verification_key: [Field; 128],
    verification_key_hash: pub Field,
    utxo_agg_proofs: [UtxoAggProof; 2],
    old_root: pub Field,
    new_root: pub Field,
    commit_hash: pub Field,
    // 6 proofs, 6 per proof
    messages: pub [Field; 36],
) {
    let mut root = old_root;

    for i in 0..2 {
        let proof = utxo_agg_proofs[i];
        let public_inputs = [
            messages[18 * i],
            messages[18 * i + 1],
            messages[18 * i + 2],
            messages[18 * i + 3],
            messages[18 * i + 4],
            messages[18 * i + 5],
            messages[18 * i + 6],
            messages[18 * i + 7],
            messages[18 * i + 8],
            messages[18 * i + 9],
            messages[18 * i + 10],
            messages[18 * i + 11],
            messages[18 * i + 12],
            messages[18 * i + 13],
            messages[18 * i + 14],
            messages[18 * i + 15],
            messages[18 * i + 16],
            messages[18 * i + 17],
            proof.old_root,
            proof.new_root,
            proof.commit_hash
        ];

        std::verify_proof_with_type(
            verification_key,
            proof.proof,
            public_inputs,
            verification_key_hash,
            1,
        );

        root = new_root;
    }

    assert(commit_hash == poseidon2::Poseidon2::hash([utxo_agg_proofs[0].commit_hash, utxo_agg_proofs[1].commit_hash], 2));
    assert(new_root == root, "Roots must match");
}
