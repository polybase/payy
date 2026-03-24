use crate::hash_merge;

#[test]
fn test_small_snapshot_with_field_reduction() {
    let left_reduced = 0x10;
    let right_reduced = 0x20;

    let result = hash_merge([left_reduced, right_reduced]);


    assert(result == 0x1bd8ab468649bc72252fa3eda6dd0793eb3db6eea57490b5629d78829833cdb3, "Hash result");
    // assert(result == 0x1dd8037c0559952dd9d36c95b8425887933e2a80481b93339bbca23af6a8e7b1, "Hash result");
}

#[test]
fn test_max_snapshot_with_field_reduction() {
    let max = 0x30644E72E131A029B85045B68181585D2833E84879B9709143E1F593F0000000;

    let result = hash_merge([max, max]);

    print("test_max_snapshot_with_field_reduction");
    print(result);

    assert(result == 0x1c359a5ebc07223748449c93b5eb57c8dc21ddfcd406034769efc98db98c2a3c, "Hash result");
}
