use poseidon::poseidon2;

pub struct Note {
    pub kind: Field,
    pub value: Field,
    pub address: Field,
    pub psi: Field,
}

pub struct InputNote {
    pub note: Note,
    pub secret_key: Field,
}

pub fn check_commitment(note: Note, commitment: Field) {
    assert(get_note_commitment(note) == commitment, "Note commitment is not valid");
}

pub fn get_note_commitment(note: Note) -> Field {
    if (note.kind == 0) {
        assert(note.value == 0, "Padding notes must be zero value");
        0
    } else {
        poseidon2::Poseidon2::hash([note.kind, note.value, note.address, note.psi], 4)
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

pub fn slice<let N: u32, let L: u32>(bytes: [u8; N], start: u32) -> [u8; L] {
    let end = start + L;
    assert(end <= N, "slice: slice end out of bounds");
    let mut result = [bytes[0]; L];
    for i in 0..L {
        result[i] = bytes[start + i];
    }
    result
}

pub fn field_from_bytes<let N: u32>(bytes: [u8; N]) -> Field {
    assert(bytes.len() < 32, "field_from_bytes: N must be less than 32");
    let mut as_field = 0;
    let mut offset = 1;
    for i in 0..N {
        as_field += (bytes[i] as Field) * offset;
        offset *= 256;
    }
    as_field
}
