// lint-long-file-override allow-max-lines=400
use crate::Element;
use bitvec::{array::BitArray, order::Msb0};
use core::num::TryFromIntError;
use ethnum::U256;
use std::str::FromStr;

macro_rules! from_int_impls {
    ($t:ty) => {
        impl From<$t> for Element {
            #[inline]
            fn from(value: $t) -> Self {
                Element(U256::from(value))
            }
        }

        impl TryFrom<Element> for $t {
            type Error = TryFromIntError;

            #[inline]
            fn try_from(value: Element) -> Result<Self, Self::Error> {
                <$t>::try_from(value.0)
            }
        }
    };
}

from_int_impls!(u8);
from_int_impls!(u16);
from_int_impls!(u32);
from_int_impls!(u64);
from_int_impls!(u128);

impl From<bool> for Element {
    #[inline]
    fn from(value: bool) -> Self {
        match value {
            false => Self::ZERO,
            true => Self::ONE,
        }
    }
}

impl FromStr for Element {
    type Err = <U256 as FromStr>::Err;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        Ok(Self(U256::from_str_radix(s, 16)?))
    }
}

impl From<U256> for Element {
    fn from(value: U256) -> Self {
        Self(value)
    }
}

impl From<Element> for U256 {
    fn from(value: Element) -> Self {
        value.0
    }
}

impl Element {
    /// Converts the Element to an array of two u128 words
    ///
    /// Returns an array where:
    /// - index 0 contains the high 128 bits
    /// - index 1 contains the low 128 bits
    #[must_use]
    pub fn to_words(self) -> [u128; 2] {
        let (high, low) = self.0.into_words();
        [high, low]
    }

    /// Creates an Element from an array of two u128 words
    ///
    /// The input array should contain:
    /// - index 0: the high 128 bits
    /// - index 1: the low 128 bits
    #[must_use]
    pub fn from_words(words: [u128; 2]) -> Self {
        Self(U256::from_words(words[0], words[1]))
    }

    /// Creates an Element from an array of four u64 values
    ///
    /// The input array should contain values in little-endian order:
    /// - arr\[0]: least significant 64 bits
    /// - arr\[1]: second least significant 64 bits
    /// - arr\[2]: second most significant 64 bits
    /// - arr\[3]: most significant 64 bits
    #[must_use]
    pub fn from_u64_array(arr: [u64; 4]) -> Self {
        let low_u128 = u128::from(arr[0]) | (u128::from(arr[1]) << 64);
        let high_u128 = u128::from(arr[2]) | (u128::from(arr[3]) << 64);
        Self(U256::from_words(high_u128, low_u128))
    }

    /// Converts the Element to an array of four u64 values
    ///
    /// Returns an array in little-endian order:
    /// - arr\[0]: least significant 64 bits
    /// - arr\[1]: second least significant 64 bits
    /// - arr\[2]: second most significant 64 bits
    /// - arr\[3]: most significant 64 bits
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_u64_array(self) -> [u64; 4] {
        let (high_u128, low_u128) = self.0.into_words();
        let arr0 = low_u128 as u64;
        let arr1 = (low_u128 >> 64) as u64;
        let arr2 = high_u128 as u64;
        let arr3 = (high_u128 >> 64) as u64;
        [arr0, arr1, arr2, arr3]
    }

    /// Convert the [`Element`] to its bytes in big-endian format
    ///
    /// ```rust
    /// # use element::Element;
    /// let element = Element::ZERO;
    /// assert_eq!(element.to_be_bytes(), [0; 32]);
    ///
    /// let element = Element::ONE;
    /// assert_eq!(element.to_be_bytes(), {
    ///     let mut temp = [0; 32];
    ///     temp[31] = 1;
    ///     temp
    /// });
    /// ```
    #[inline]
    #[must_use]
    pub fn to_be_bytes(self) -> [u8; 32] {
        self.0.to_be_bytes()
    }

    /// Convert the [`Element`] to its bits in big-endian format
    ///
    /// ```rust
    /// # use element::Element;
    /// let element = Element::ZERO;
    /// let bits = element.to_be_bits();
    ///
    /// ```
    #[inline]
    #[must_use]
    pub fn to_be_bits(self) -> BitArray<[u8; 32], Msb0> {
        let bits = self.0.to_be_bytes();
        BitArray::new(bits)
    }

    /// Convert the [`Element`] to its bytes in little-endian format
    ///
    /// ```rust
    /// # use element::Element;
    /// let element = Element::ZERO;
    /// assert_eq!(element.to_le_bytes(), [0; 32]);
    ///
    /// let element = Element::ONE;
    /// assert_eq!(element.to_le_bytes(), {
    ///     let mut temp = [0; 32];
    ///     temp[0] = 1;
    ///     temp
    /// });
    /// ```
    #[inline]
    #[must_use]
    pub fn to_le_bytes(self) -> [u8; 32] {
        self.0.to_le_bytes()
    }

    /// Convert big-endian bytes into an [`Element`]
    /// ```rust
    /// # use element::Element;
    /// let element = Element::from_be_bytes([0; 32]);
    /// assert_eq!(element, Element::ZERO);
    ///
    /// let element = Element::from_be_bytes({
    ///     let mut temp = [0; 32];
    ///     temp[31] = 1;
    ///     temp
    /// });
    /// assert_eq!(element, Element::ONE);
    /// ```
    #[inline]
    #[must_use]
    pub fn from_be_bytes(bytes: [u8; 32]) -> Self {
        Self(U256::from_be_bytes(bytes))
    }

    /// Convert little-endian bytes into an [`Element`]
    /// ```rust
    /// # use element::Element;
    /// let element = Element::from_le_bytes([0; 32]);
    /// assert_eq!(element, Element::ZERO);
    ///
    /// let element = Element::from_le_bytes({
    ///     let mut temp = [0; 32];
    ///     temp[0] = 1;
    ///     temp
    /// });
    /// assert_eq!(element, Element::ONE);
    /// ```
    #[inline]
    #[must_use]
    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        Self(U256::from_le_bytes(bytes))
    }

    /// Decomposes an [`Element`] into two [`Element`]s, low and high, in big-endian order.
    ///
    /// The low field contains the lower 16 bytes (128 bits) of the input field and
    /// the high field contains the upper 16 bytes (128 bits) of the input field.
    ///
    /// ```rust
    /// # use element::Element;
    /// let element = Element::from(0x0123456789ABCDEFu64);
    /// let (high, low) = element.decompose_be();
    /// assert_eq!(high, Element::ZERO);
    /// assert_eq!(low, Element::from(0x0123456789ABCDEFu64));
    /// ```
    #[inline]
    #[must_use]
    pub fn decompose_be(self) -> (Element, Element) {
        let bytes = self.to_be_bytes();
        let high_bytes: [u8; 16] = bytes[..16].try_into().unwrap();
        let low_bytes: [u8; 16] = bytes[16..].try_into().unwrap();

        let high = Element::from(U256::from_be_bytes(
            [0u8; 16]
                .into_iter()
                .chain(high_bytes)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        ));
        let low = Element::from(U256::from_be_bytes(
            [0u8; 16]
                .into_iter()
                .chain(low_bytes)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        ));

        (high, low)
    }

    /// Decomposes an [`Element`] into two [`Element`]s, low and high, in little-endian order.
    ///
    /// The low field contains the lower 16 bytes (128 bits) of the input field and
    /// the high field contains the upper 16 bytes (128 bits) of the input field.
    ///
    /// ```rust
    /// # use element::Element;
    /// let element = Element::from(0x0123456789ABCDEFu64);
    /// let (high, low) = element.decompose_le();
    /// assert_eq!(high, Element::ZERO);
    /// assert_eq!(low, Element::from(0x0123456789ABCDEFu64));
    /// ```
    #[inline]
    #[must_use]
    pub fn decompose_le(self) -> (Element, Element) {
        let bytes = self.to_le_bytes();
        let low_bytes: [u8; 16] = bytes[..16].try_into().unwrap();
        let high_bytes: [u8; 16] = bytes[16..].try_into().unwrap();

        let high = Element::from(U256::from_le_bytes(
            high_bytes
                .into_iter()
                .chain([0u8; 16])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        ));
        let low = Element::from(U256::from_le_bytes(
            low_bytes
                .into_iter()
                .chain([0u8; 16])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        ));

        (high, low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_be_bits() {
        let element = Element::ZERO;
        let bits = BitArray::<[u8; 32], Msb0>::new([0u8; 32]);
        assert_eq!(element.to_be_bits(), bits);

        let element = Element::ONE;
        assert_eq!(
            element
                .to_be_bits()
                .iter()
                .rev()
                .take(1)
                .collect::<Vec<_>>(),
            vec![true]
        );
    }

    #[test]
    fn test_from_str() {
        assert_eq!(Element::from_str("0").unwrap(), Element::ZERO);
        assert_eq!(Element::from_str("0x0").unwrap(), Element::ZERO);
        assert_eq!(Element::from_str("0x1").unwrap(), Element::ONE);
        assert_eq!(Element::from_str("0xB").unwrap(), Element::from(11u64));
    }
}
