use ethnum::{uint, U256};
use ff::PrimeField;

use crate::{hash_merge, Base, Element};

impl Element {
    /// The modulus of the underlying prime field
    pub const MODULUS: Element = Element(uint!(
        "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001"
    ));

    /// Return the result of hash-merging this value with `other`
    ///
    /// This element is considered to be on the left:
    /// ```rust
    /// # use zk_primitives::*;
    /// let a = Element::new(1);
    /// let b = Element::new(2);
    ///
    /// let ab = a.hashed_with(b);
    ///
    /// assert_eq!(ab, hash_merge([a, b]));
    /// ```
    #[inline]
    #[must_use = "this function doesn't modify self"]
    pub fn hashed_with(self, other: Element) -> Self {
        hash_merge([self, other])
    }

    /// Convert this [`Element`] to its equivalent [`Base`] representation
    #[inline]
    #[must_use]
    pub fn to_base(self) -> Base {
        let u8s = self.0.to_le_bytes();
        Base::from_raw(u8s_to_u64(u8s))
    }

    /// Create an [`Element`] from a [`Base`]
    #[inline]
    #[must_use]
    pub fn from_base(base: Base) -> Element {
        let u8s = base.to_repr();
        Self(U256::from_le_bytes(u8s))
    }

    /// Reduce this element to its canonical form
    ///
    /// [`Base`]s are integers modulo "some prime number", and as such have a smaller set of
    /// possible values than [`Element`], which is just a 256-bit unsigned integer.
    ///
    /// This function reduces an element to its canonical form by applying this modulus.
    ///
    /// Elements in canonical form are guaranteed to be unchanged when converting to/from a [`Base`]
    #[inline]
    pub fn canonicalize(&mut self) {
        self.0 %= Self::MODULUS.0;
    }

    /// Whether this [`Element`] is in its canonical form
    ///
    /// See the docs for [`Element::canonicalize`] for more details on what the canonical form of
    /// an [`Element`] is
    #[inline]
    #[must_use]
    pub fn is_canonical(&self) -> bool {
        let mut canonical = *self;
        canonical.canonicalize();
        self == &canonical
    }
}

impl From<Base> for Element {
    fn from(value: Base) -> Self {
        Element::from_base(value)
    }
}

impl From<Element> for Base {
    fn from(value: Element) -> Self {
        value.to_base()
    }
}

fn u8s_to_u64(u8s: [u8; 32]) -> [u64; 4] {
    [
        u64::from_le_bytes((&u8s[0..8]).try_into().unwrap()),
        u64::from_le_bytes((&u8s[8..16]).try_into().unwrap()),
        u64::from_le_bytes((&u8s[16..24]).try_into().unwrap()),
        u64::from_le_bytes((&u8s[24..32]).try_into().unwrap()),
    ]
}

#[cfg(test)]
fn u64s_to_u8s(u64s: [u64; 4]) -> [u8; 32] {
    core::array::from_fn(|i| {
        let u64 = u64s[i / 8];
        u64.to_le_bytes()[i % 8]
    })
}

#[cfg(test)]
mod tests {
    use test_strategy::proptest;

    use super::*;

    #[proptest]
    fn u64_u8_conversion(u64s: [u64; 4]) {
        let u8s = u64s_to_u8s(u64s);
        let u64s_again = u8s_to_u64(u8s);

        assert_eq!(u64s, u64s_again);
    }

    #[proptest]
    fn to_from_base_biject(mut element: Element) {
        element.canonicalize();

        let base = element.to_base();
        let element_again = Element::from_base(base);

        assert_eq!(element, element_again);
    }
}
