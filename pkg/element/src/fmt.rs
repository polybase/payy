use std::fmt::{Binary, Debug, Display, LowerExp, LowerHex, Octal, UpperExp, UpperHex};

use crate::Element;
use ethnum::U256;

macro_rules! fmt_impl {
    ($t:ident, $u:ident) => {
        impl $u for Element {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                <U256 as $t>::fmt(&self.0, f)
            }
        }
    };
    ($t:ident) => {
        fmt_impl!($t, $t);
    };
}

fmt_impl!(LowerHex, Display);
fmt_impl!(LowerHex, Debug);
fmt_impl!(UpperHex);
fmt_impl!(LowerHex);
fmt_impl!(UpperExp);
fmt_impl!(LowerExp);
fmt_impl!(Binary);
fmt_impl!(Octal);
