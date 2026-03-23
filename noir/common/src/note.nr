use poseidon::poseidon2;
use crate::{InputNote, Note};

pub fn check_commitment(note: Note, commitment: Field) {
    assert(get_note_commitment(note) == commitment, "Note commitment is not valid");
}

pub fn get_note_commitment(note: Note) -> Field {
    if (note.kind == 0) {
        assert(note.value == 0, "Padding notes must be zero value");
        0
    } else {
        poseidon2::Poseidon2::hash(
            [0x2, note.kind, note.value, note.address, note.psi, 0, 0],
            7,
        )
    }
}

pub fn get_address(secret_key: Field) -> Field {
    poseidon2::Poseidon2::hash([secret_key, 0], 2)
}

pub fn check_input_note_ownership(input_note: InputNote) {
    if (input_note.note.value != 0) {
        assert(
            get_address(input_note.secret_key) == input_note.note.address,
            "Input note is not owned by the owner",
        );
    }
}

pub fn is_multiple_kinds(notes: [Note; 4]) -> (bool, Field) {
    let mut first_non_zero_kind = 0;
    let mut is_multiple_kinds = false;

    for i in 0..4 {
        if !is_multiple_kinds {
            if notes[i].kind != 0 {
                if first_non_zero_kind == 0 {
                    first_non_zero_kind = notes[i].kind;
                } else if first_non_zero_kind != notes[i].kind {
                    is_multiple_kinds = true;
                }
            }
        }
    }

    (is_multiple_kinds, first_non_zero_kind)
}
