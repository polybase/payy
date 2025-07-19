fn radix(field1: Field) {
    let bytes: [u8; 8] = field1.to_be_bytes();
    assert_eq(bytes, [0, 0, 0, 0, 0, 0, 1, 3]);
    assert_eq(Field::from_be_bytes::<8>(bytes), field1);
}

fn radix2(field1: Field) {
    let bytes: [u8; 2] = field1.to_be_bytes();
    assert_eq(bytes, [1, 3]);
    assert_eq(Field::from_be_bytes::<2>(bytes), field1);
}

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
    radix(259);
    radix2(259);
    to_bytes_3(259)
}

#[test]
fn test_max_value() {
    to_bytes_256(259);
}
