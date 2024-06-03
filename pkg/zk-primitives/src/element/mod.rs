use ethnum::U256;

mod arith;
mod collision;
mod convert;
mod fmt;
mod halo2;
mod lsb;

#[cfg(feature = "borsh")]
mod borsh_impls;

#[cfg(feature = "rand")]
mod rand_impls;
#[cfg(feature = "rand")]
pub use rand_impls::Insecure;

pub use lsb::Lsb;

#[cfg(feature = "serde")]
mod serde;

/// A 256-bit unsigned integer
///
/// This type is a wrapper around a [`U256`], so can represent any value in the range `0..=(2^256 -
/// 1)`.
/// However, in a ZK context, it is usually converted to a [`Base`], which is an integer modulo
/// "some large prime". This restricts the set of usable values to something approximating a `u254`
///
/// [`Base`]: crate::Base
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Element(#[cfg_attr(feature = "serde", serde(with = "serde"))] pub(crate) U256);

impl Element {
    /// The zero element of the group (the additive identity)
    pub const ZERO: Self = Self(U256::ZERO);

    /// The one element of the group (the multiplicative identity)
    pub const ONE: Self = Self(U256::ONE);

    /// The largest possible element (note that this is not canonical)
    pub const MAX: Self = Self(U256::MAX);

    /// A null hash value (this is identical to [`Element::ZERO`])
    ///
    /// Note that this value is chosen arbitrarily
    pub const NULL_HASH: Self = Self::ZERO;

    /// The [`Element`] used as the initialization vector when hashing bytes
    pub(crate) const BYTE_HASH_IV: Self = Self(U256::new(2));

    /// Create a new [`Element`] from a u64
    ///
    /// This is largely provided to help type inference in simple cases
    #[inline]
    #[must_use]
    pub fn new(i: u64) -> Self {
        Self(U256::from(i))
    }

    /// Attempt to convert this [`Element`] to a bool
    ///
    /// If this value is not 0 or 1, `None` is returned
    #[inline]
    #[must_use]
    pub fn as_bool(self) -> Option<bool> {
        match self {
            Self::ZERO => Some(false),
            Self::ONE => Some(true),
            _else => None,
        }
    }

    /// Convert this [`Element`] to a U256 string
    #[inline]
    #[must_use]
    pub fn to_u256(self) -> U256 {
        self.0
    }

    /// Convert this [`Element`] to a hex string
    #[inline]
    #[must_use]
    pub fn to_hex(self) -> String {
        hex::encode(self.to_be_bytes())
    }

    /// If this element is zero, returns true
    #[inline]
    #[must_use]
    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }
}

macro_rules! partial_eq_impl {
    ($int:ty) => {
        impl PartialEq<$int> for Element {
            fn eq(&self, other: &$int) -> bool {
                *self == Element::from(*other)
            }
        }
    };
}

partial_eq_impl!(bool);
partial_eq_impl!(u8);
partial_eq_impl!(u16);
partial_eq_impl!(u32);
partial_eq_impl!(u64);
partial_eq_impl!(u128);

#[cfg(any(test, feature = "proptest"))]
pub mod proptest {
    use super::Element;
    use ::proptest::{arbitrary::StrategyFor, prelude::*, strategy::Map};
    use ethnum::U256;

    impl Arbitrary for Element {
        type Strategy = Map<StrategyFor<[u8; 32]>, fn([u8; 32]) -> Self>;
        type Parameters = ();

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<[u8; 32]>().prop_map(|array| Self(U256::from_be_bytes(array)))
        }
    }
}

#[cfg(test)]
mod test {
    use super::Element;

    #[test]
    fn syntax_test() {
        let element = Element::new(123);

        assert_eq!(element + 1u64, Element::new(124));
        assert_eq!(element * 2u64, Element::new(246));
        assert_eq!(element - 2u64, Element::new(121));
        assert_eq!(element + Element::ONE, Element::new(124));
        assert_eq!(element * Element::new(2), Element::new(246));
        assert_eq!(element - Element::new(2), Element::new(121));

        assert_eq!(Element::new(1).to_string(), "1");
        assert_eq!(Element::new(100).to_string(), "64");
        assert_eq!(Element::new(123).to_string(), "7b");

        assert_eq!(
            (1..=10).map(Element::new).sum::<Element>(),
            Element::new(55)
        );

        assert_eq!(
            (1..=5).map(Element::new).product::<Element>(),
            Element::new(120)
        );
    }
}
