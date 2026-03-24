use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::Deref,
};

use bitvec::{
    prelude::{BitArray, Msb0},
    slice::BitSlice,
};

use crate::Element;

/// A handle to the `N` least significant bits of an element
///
/// See [`Element::lsb`] for more details
#[derive(Debug, Clone, Copy)]
#[doc(alias = "least_significant_bits")]
pub struct Lsb {
    /// All the bits of the element in big-endian order (i.e. most significant bits first)
    bits: BitArray<[u8; 32], Msb0>,
    count: usize,
}

impl Lsb {
    /// Get the bits as a [`BitSlice`]
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &BitSlice<u8, Msb0> {
        let start_index = self.bits.len() - self.count;
        &self.bits[start_index..]
    }
}

impl PartialEq for Lsb {
    #[inline]
    fn eq(&self, other: &Lsb) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for Lsb {}

impl PartialOrd for Lsb {
    #[inline]
    fn partial_cmp(&self, other: &Lsb) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Lsb {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.deref().cmp(other.as_slice())
    }
}

impl Hash for Lsb {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl Deref for Lsb {
    type Target = BitSlice<u8, Msb0>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl IntoIterator for Lsb {
    type Item = bool;
    type IntoIter = core::iter::Skip<bitvec::array::IntoIter<[u8; 32], Msb0>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let skip = self.bits.len() - self.count;
        self.bits.into_iter().skip(skip)
    }
}

impl IntoIterator for &Lsb {
    type Item = bool;
    type IntoIter = core::iter::Skip<bitvec::array::IntoIter<[u8; 32], Msb0>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (*self).into_iter()
    }
}

impl Element {
    /// Get the `count` least significant bits in big-endian order (i.e. with the most significant
    /// bits first)
    ///
    /// ```rust
    /// # use element::Element;
    /// let element = Element::new(5);  // 0b000...000101
    /// let bits = element.lsb(4);
    /// let bits: Vec<bool> = bits.into_iter().collect();
    ///
    /// assert_eq!(bits, vec![false, true, false, true]);
    /// ```
    #[doc(alias = "least_significant_bits")]
    #[inline]
    #[must_use]
    pub fn lsb(&self, count: usize) -> Lsb {
        let bits = self.0.to_be_bytes();
        let bits = BitArray::new(bits);
        Lsb { bits, count }
    }
}

#[cfg(test)]
mod tests {
    use test_strategy::proptest;

    use super::*;

    #[test]
    fn eq_ignores_upper_bits() {
        let a = Element::new(5);
        let b = a + (1u64 << 20);

        assert_eq!(a.lsb(4), b.lsb(4));
        assert_eq!(a.lsb(10), b.lsb(10));

        assert_ne!(a.lsb(22), b.lsb(22));
    }

    #[proptest]
    fn lsb_has_right_number_of_bits(element: Element, #[strategy(0usize..=256)] num_bits: usize) {
        let bits = element.lsb(num_bits);

        assert_eq!(bits.len(), num_bits);
        assert_eq!(bits.iter().collect::<Vec<_>>().len(), num_bits);
        assert_eq!(bits.into_iter().collect::<Vec<_>>().len(), num_bits);
    }
}
