use std::ops::{Shl, ShlAssign, Shr, ShrAssign};

use ethnum::U256;

use crate::Element;

/// Implement a binary operation
macro_rules! binop {
    ($trait:ident, $f:ident, $($t:tt)*) => {
        impl core::ops::$trait<Element> for Element {
            type Output = Element;

            #[inline]
            fn $f(self, rhs: Element) -> Self::Output {
                Element(self.0 $($t)* rhs.0)
            }
        }

        impl core::ops::$trait<u8> for Element {
            type Output = Element;

            #[inline]
            fn $f(self, rhs: u8) -> Self::Output {
                self $($t)* Element::from(rhs)
            }
        }

        impl core::ops::$trait<u16> for Element {
            type Output = Element;

            #[inline]
            fn $f(self, rhs: u16) -> Self::Output {
                self $($t)* Element::from(rhs)
            }
        }

        impl core::ops::$trait<u32> for Element {
            type Output = Element;

            #[inline]
            fn $f(self, rhs: u32) -> Self::Output {
                self $($t)* Element::from(rhs)
            }
        }

        impl core::ops::$trait<u64> for Element {
            type Output = Element;

            #[inline]
            fn $f(self, rhs: u64) -> Self::Output {
                self $($t)* Element::from(rhs)
            }
        }

        impl core::ops::$trait<u128> for Element {
            type Output = Element;

            #[inline]
            fn $f(self, rhs: u128) -> Self::Output {
                self $($t)* Element::from(rhs)
            }
        }

    };
}

binop!(Add, add, +);
binop!(Sub, sub, -);
binop!(Mul, mul, *);
binop!(Div, div, /);
binop!(Rem, rem, %);

binop!(BitXor, bitxor, ^);
binop!(BitOr, bitor, |);
binop!(BitAnd, bitand, &);

impl Shl<u8> for Element {
    type Output = Element;

    fn shl(self, rhs: u8) -> Self::Output {
        U256::shl(self.0, rhs).into()
    }
}

impl Shr<u8> for Element {
    type Output = Element;

    fn shr(self, rhs: u8) -> Self::Output {
        U256::shr(self.0, rhs).into()
    }
}

impl ShlAssign<u8> for Element {
    fn shl_assign(&mut self, rhs: u8) {
        self.0 <<= rhs;
    }
}

impl ShrAssign<u8> for Element {
    fn shr_assign(&mut self, rhs: u8) {
        self.0 >>= rhs;
    }
}

impl Shl for Element {
    type Output = Element;

    fn shl(self, rhs: Self) -> Self::Output {
        U256::shl(self.0, rhs.0).into()
    }
}

impl Shr for Element {
    type Output = Element;

    fn shr(self, rhs: Self) -> Self::Output {
        U256::shr(self.0, rhs.0).into()
    }
}

impl ShlAssign for Element {
    fn shl_assign(&mut self, rhs: Self) {
        self.0 <<= rhs.0;
    }
}

impl ShrAssign for Element {
    fn shr_assign(&mut self, rhs: Self) {
        self.0 >>= rhs.0;
    }
}

impl core::iter::Sum<Element> for Element {
    fn sum<I: Iterator<Item = Element>>(iter: I) -> Self {
        iter.fold(Element::ZERO, |a, b| a + b)
    }
}

impl core::iter::Product<Element> for Element {
    fn product<I: Iterator<Item = Element>>(iter: I) -> Self {
        iter.fold(Element::ONE, |a, b| a * b)
    }
}
