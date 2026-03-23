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

pub fn is_one_of(value: Field, one_of: [Field]) -> bool {
    let mut is_one_of = false;
    for possible in one_of {
        if possible == value {
            is_one_of = true;
        }
    }
    is_one_of
}
