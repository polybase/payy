use bitvec::prelude::*;
pub use bitvec::view::BitViewSized;
use halo2_base::halo2_proofs::halo2curves::bn256::Fr;
use halo2_base::halo2_proofs::halo2curves::group::ff::PrimeField;

pub type FieldBits<V> = BitArray<V, Lsb0>;

pub const MODULUS: [u64; 4] = [
    0x43e1f593f0000001,
    0x2833e84879b97091,
    0xb85045b68181585d,
    0x30644e72e131a029,
];

pub trait PrimeFieldBits: PrimeField {
    /// The backing store for a bit representation of a prime field element.
    type ReprBits: BitViewSized + Send + Sync;

    /// Converts an element of the prime field into a little-endian sequence of bits.
    fn to_le_bits(&self) -> FieldBits<Self::ReprBits>;

    /// Returns the bits of the field characteristic (the modulus) in little-endian order.
    fn char_le_bits() -> FieldBits<Self::ReprBits>;
}

impl PrimeFieldBits for Fr {
    type ReprBits = [u64; 4];

    fn to_le_bits(&self) -> FieldBits<Self::ReprBits> {
        let bytes = self.to_repr();
        let limbs = [
            u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        ];

        FieldBits::new(limbs)
    }

    fn char_le_bits() -> FieldBits<Self::ReprBits> {
        FieldBits::new(MODULUS)
    }
}
