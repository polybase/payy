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
    /// Convert the [`Element`] to its bytes in big-endian format
    ///
    /// ```rust
    /// # use zk_primitives::*;
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
    /// # use zk_primitives::*;
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
    /// # use zk_primitives::*;
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
    /// # use zk_primitives::*;
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
    /// # use zk_primitives::*;
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
