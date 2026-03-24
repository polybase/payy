use crate::{Base, Element};
use acvm::AcirField;
use ethnum::uint;

impl Element {
    pub const MODULUS: Element = Element(uint!(
        "0x30644E72E131A029B85045B68181585D2833E84879B9709143E1F593F0000001"
    ));

    /// Create an [`Element`] from a [`Base`]
    #[inline]
    #[must_use]
    pub fn from_base(base: Base) -> Element {
        let bytes: [u8; 32] = base.to_be_bytes().try_into().unwrap();
        Element::from_be_bytes(bytes)
    }

    /// Convert this [`Element`] to its equivalent [`Base`] representation
    #[inline]
    #[must_use]
    pub fn to_base(&self) -> Base {
        Base::from_be_bytes_reduce(self.to_be_bytes().as_slice())
    }
}

impl From<Base> for Element {
    #[inline]
    fn from(base: Base) -> Self {
        Self::from_base(base)
    }
}

impl From<Element> for Base {
    #[inline]
    fn from(element: Element) -> Self {
        element.to_base()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_and_from_base() {
        // Test with a simple value
        let element = Element::from(42u64);
        let base = element.to_base();
        let element_from_base = Element::from_base(base);
        assert_eq!(element, element_from_base);

        // Test with a larger value
        let element = Element::from(u64::MAX);
        let base = element.to_base();
        let element_from_base = Element::from_base(base);
        assert_eq!(element, element_from_base);

        // Test with a value close to the modulus
        let element = Element::MODULUS - Element::from(1u64);
        let base = element.to_base();
        let element_from_base = Element::from_base(base);
        assert_eq!(element, element_from_base);

        // Test with zero
        let element = Element::ZERO;
        let base = element.to_base();
        let element_from_base = Element::from_base(base);
        assert_eq!(element, element_from_base);
    }

    #[test]
    fn test_base_roundtrip_consistency() {
        // Create a random element
        let original = Element::from(0x1234_5678_90ab_cdef_u64);

        // Convert to base and back
        let base = original.to_base();
        let roundtrip = Element::from_base(base);

        // Should be the same
        assert_eq!(original, roundtrip);
    }
}
