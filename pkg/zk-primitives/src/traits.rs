/// Trait for types that can be converted to a byte representation.
pub trait ToBytes {
    /// Convert to bytes
    fn to_bytes(&self) -> Vec<u8>;
}
