fn to_bytes_3(field1: Field) {
    let bytes: [u8; 3] = field1.to_be_bytes();

    assert_eq(bytes, [0, 1, 3]);
    assert_eq(Field::from_be_bytes::<3>(bytes), field1);
}

fn to_bytes_256(field1: Field) {
    let bytes: [u8; 31] = field1.to_be_bytes();
    assert_eq(
        bytes,
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 3,
        ],
    );
}

#[test]
fn test_main() {
    to_bytes_3(259)
}

#[test]
fn test_max_value() {
    to_bytes_256(259);
}
