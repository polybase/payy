use crate::hash_merge;
use crate::poseidon::{hash_2, hash, permute};

#[test]
fn test_hash_zeros() {
    // Test hash([0, 0]) from snapshot
    let result = hash_merge([0, 0]);
    // Expected: 0x2ba00861b8f1581f5e17d438e323fa2809f58f1a60009dcd05edb1c9c7c833da
    let expected = 0x2ba00861b8f1581f5e17d438e323fa2809f58f1a60009dcd05edb1c9c7c833da;
    assert(result == expected, "Hash of [0, 0] mismatch");
}

#[test]
fn test_hash_zero_one() {
    // Test hash([0, 1]) from snapshot
    let result = hash_merge([0, 1]);
    // Expected: 0x1f4c45f9764aa17b0b277e23e1da0cefd69affc8caa18bb01f65b5c9d9761d21
    let expected = 0x1f4c45f9764aa17b0b277e23e1da0cefd69affc8caa18bb01f65b5c9d9761d21;
    assert(result == expected, "Hash of [0, 1] mismatch");
}

#[test]
fn test_hash_one_zero() {
    // Test hash([1, 0]) from snapshot
    let result = hash_merge([1, 0]);
    // Expected: 0x034797ee520c67ec5ee6fd37f581ba7267449a21de06aa55a09821caeec2e89d
    let expected = 0x034797ee520c67ec5ee6fd37f581ba7267449a21de06aa55a09821caeec2e89d;
    assert(result == expected, "Hash of [1, 0] mismatch");
}

#[test]
fn test_hash_consistency() {
    // Test that hash_2 and hash_merge produce the same results
    let a = 42;
    let b = 123;

    let result1 = hash_merge([a, b]);
    let result2 = hash_2(a, b);
    let result3 = hash([a, b]);

    assert(result1 == result2, "hash_merge and hash_2 should produce same result");
    assert(result1 == result3, "hash_merge and hash should produce same result");
}

#[test]
fn test_hash_non_symmetric() {
    // Test that hash is not symmetric
    let a = 10;
    let b = 20;

    let hash_ab = hash_merge([a, b]);
    let hash_ba = hash_merge([b, a]);

    assert(hash_ab != hash_ba, "Hash should not be symmetric");
}

#[test]
fn test_permutation_basic() {
    // Test the permutation function with simple input
    let input = [1, 2, 3];
    let output = permute(input);

    // Just verify it doesn't crash and produces different output
    assert(output[0] != input[0], "Permutation should change the state");
    assert(output[1] != input[1], "Permutation should change the state");
    assert(output[2] != input[2], "Permutation should change the state");
}

#[test]
fn test_hash_single_element() {
    // Test hashing a single element
    let result = hash([42]);
    // Verify it produces a valid field element (non-zero in this case)
    assert(result != 0, "Single element hash should not be zero");
}

#[test]
fn test_hash_three_elements() {
    // Test hashing three elements
    let result = hash([1, 2, 3]);
    // Verify it produces a valid field element
    assert(result != 0, "Three element hash should not be zero");

    // Also test that order matters
    let result2 = hash([3, 2, 1]);
    assert(result != result2, "Hash should depend on element order");
}

#[test]
fn test_small_sequential_values() {
    // Test with small sequential values
    for i in 1..10 {
        let result = hash_merge([i as Field, (i + 1) as Field]);
        let result_reversed = hash_merge([(i + 1) as Field, i as Field]);

        // Verify results are deterministic (same input gives same output)
        let result2 = hash_merge([i as Field, (i + 1) as Field]);
        assert(result == result2, "Hash should be deterministic");

        // Verify non-symmetry
        assert(result != result_reversed, "Hash should not be symmetric");
    }
}

#[test]
fn test_powers_of_two() {
    // Test with powers of 2
    let mut prev_hash = 0;
    for i in 0..10 {
        let power = 1 << i; // 2^i
        let result = hash_merge([power as Field, 0]);

        // Each power of 2 should give a different hash
        assert(result != prev_hash, "Different inputs should give different hashes");
        prev_hash = result;
    }
}

#[test]
fn test_large_safe_values() {
    // Test with large values that are safely within field range
    let large1 = 0x1234567890abcdef1234567890abcdef;
    let large2 = 0xfedcba0987654321fedcba0987654321;

    let result1 = hash_merge([large1, large2]);
    let result2 = hash_merge([large2, large1]);

    assert(result1 != result2, "Hash should not be symmetric for large values");
}

#[test]
fn test_hash_chain() {
    // Test chaining hashes
    let initial = hash_merge([1, 2]);
    let second = hash_merge([initial, 3]);
    let third = hash_merge([second, 4]);

    // Verify different chain gives different result
    let alt_initial = hash_merge([2, 1]);
    let alt_second = hash_merge([alt_initial, 3]);
    let alt_third = hash_merge([alt_second, 4]);

    assert(third != alt_third, "Different hash chains should give different results");
}

#[test]
fn test_boundary_values() {
    // Test with boundary values
    let max_u32 = 0xffffffff;
    let max_u64 = 0xffffffffffffffff;
    let max_u128 = 0xffffffffffffffffffffffffffffffff;

    let result1 = hash_merge([0, max_u32]);
    let result2 = hash_merge([0, max_u64]);
    let result3 = hash_merge([0, max_u128]);

    // All should produce different hashes
    assert(result1 != result2, "Different boundary values should produce different hashes");
    assert(result2 != result3, "Different boundary values should produce different hashes");
    assert(result1 != result3, "Different boundary values should produce different hashes");
}

#[test]
fn test_collision_resistance() {
    // Test that similar inputs don't collide
    let base = 0x123456789;

    let result1 = hash_merge([base, base]);
    let result2 = hash_merge([base, base + 1]);
    let result3 = hash_merge([base + 1, base]);
    let result4 = hash_merge([base + 1, base + 1]);

    // All combinations should be different
    assert(result1 != result2, "Different inputs should not collide");
    assert(result1 != result3, "Different inputs should not collide");
    assert(result1 != result4, "Different inputs should not collide");
    assert(result2 != result3, "Different inputs should not collide");
    assert(result2 != result4, "Different inputs should not collide");
    assert(result3 != result4, "Different inputs should not collide");
}
