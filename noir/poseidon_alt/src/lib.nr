mod constants;
mod poseidon;
mod test;
mod test_field_snapshot;

// Re-export the main hash functions
pub use poseidon::{hash_2, hash, permute, PoseidonSponge};

// Main hash_merge function to match the Rust implementation signature
// This is the primary function used by hash-poseidon
pub fn hash_merge(elements: [Field; 2]) -> Field {
    hash_2(elements[0], elements[1])
}

// Additional convenience functions
pub fn hash_single(input: Field) -> Field {
    hash([input])
}

pub fn hash_three(a: Field, b: Field, c: Field) -> Field {
    hash([a, b, c])
}

pub fn hash_four(a: Field, b: Field, c: Field, d: Field) -> Field {
    hash([a, b, c, d])
}
