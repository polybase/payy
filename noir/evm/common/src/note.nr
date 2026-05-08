use poseidon::poseidon2::Poseidon2;

// SPEC(docs/specs/privacy-protocol#note-six-field-structure)
// SPEC(docs/specs/privacy-protocol#note-field-definitions-table)
pub struct EvmNote {
    pub kind: Field,
    pub token: Field,
    pub nonce: Field,
    pub psi: Field,
    pub owner: Field,
    pub value: Field,
}

// SPEC(docs/specs/privacy-protocol#design-principle-protocol-local-ownership)
// INVARIANT(docs/specs/privacy-protocol#owner-from-grumpkin-pk)
// SPEC(docs/specs/privacy-protocol#owner-derivation-note-types)
pub fn owner_from_public_key(public_key_x: Field, public_key_y: Field) -> Field {
    Poseidon2::hash([public_key_x, public_key_y], 2)
}

// SPEC(docs/specs/privacy-protocol#note-commitment-poseidon-six)
// SPEC(docs/specs/privacy-protocol#padding-note-shape-and-commitment)
pub fn note_commitment(note: EvmNote) -> Field {
    if note.kind == 0 {
        // INVARIANT(docs/specs/privacy-protocol#padding-note-all-zero)
        assert(note.token == 0, "padding token");
        assert(note.nonce == 0, "padding nonce");
        assert(note.psi == 0, "padding psi");
        assert(note.owner == 0, "padding owner");
        assert(note.value == 0, "padding value");
        0
    } else {
        // SPEC(docs/specs/privacy-protocol#zero-value-note-shape)
        // SPEC(docs/specs/privacy-protocol#incoming-note-not-distinguished-at-commitment)
        assert(note.kind == 1, "invalid note kind");
        Poseidon2::hash(
            [
                note.kind,
                note.token,
                note.nonce,
                note.psi,
                note.owner,
                note.value,
            ],
            6,
        )
    }
}

// SPEC(docs/specs/privacy-protocol#nullifier-poseidon-commit-psi)
// INVARIANT(docs/specs/privacy-protocol#nullifier-unlinkable-via-private-psi)
pub fn note_nullifier(commitment: Field, psi: Field) -> Field {
    if commitment == 0 {
        0
    } else {
        Poseidon2::hash([commitment, psi], 2)
    }
}

#[test]
fn padding_note_commitment_is_zero() {
    let note = EvmNote { kind: 0, token: 0, nonce: 0, psi: 0, owner: 0, value: 0 };
    assert(note_commitment(note) == 0);
}

#[test]
fn live_note_commitment_is_non_zero() {
    let note = EvmNote { kind: 1, token: 9, nonce: 3, psi: 7, owner: 11, value: 42 };
    assert(note_commitment(note) != 0);
}
