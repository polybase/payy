use poseidon::poseidon2::Poseidon2;

// SPEC(docs/specs/privacy-protocol#design-principle-note-chain-convention)
//
// SPEC(docs/specs/privacy-protocol#nonce-hash-formula)
// INVARIANT(docs/specs/privacy-protocol#first-note-input-psi-zero)
pub fn first_nonce_hash(kind: Field, token: Field, owner: Field) -> Field {
    Poseidon2::hash([kind, token, owner, 0, 0], 5)
}

// SPEC(docs/specs/privacy-protocol#nonce-hash-formula)
// SPEC(docs/specs/privacy-protocol#nonce-hash-chain-requires-prev-psi)
pub fn next_nonce_hash(
    kind: Field,
    token: Field,
    owner: Field,
    output_nonce: Field,
    input_psi: Field,
) -> Field {
    Poseidon2::hash([kind, token, owner, output_nonce, input_psi], 5)
}

#[test]
fn first_nonce_hash_is_stable() {
    assert(first_nonce_hash(1, 22, 33) == first_nonce_hash(1, 22, 33));
}
