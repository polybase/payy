//! Types and traits exposed to the `noir-abi-inputs-macro` proc-macro crate.
//!
//! The traits defined in this module are implemented by structs that
//! are generated from proof JSONs.

use acvm::AcirField;
use std::fmt::Debug;

pub use element::{Base, Element};
use noirc_abi::InputMap;
use noirc_driver::CompiledProgram;

pub trait ProofInputs {
    const PROGRAM: &'static str;
    const KEY: &'static [u8];
    fn bytecode(&self) -> &[u8];
    fn compiled_program(&self) -> &CompiledProgram;

    fn input_map(&self) -> InputMap;

    type PublicInputs: PublicInputs;
}

pub trait PublicInputs: ToFields + FromFields {
    const KEY: &'static [u8];
    const ORACLE_HASH_KECCAK: bool;
}

pub trait ToFields {
    fn to_fields(&self, out: &mut Vec<Base>);
}

impl ToFields for Element {
    fn to_fields(&self, out: &mut Vec<Base>) {
        out.push(self.to_base())
    }
}

impl<const N: usize, T: ToFields> ToFields for [T; N] {
    fn to_fields(&self, out: &mut Vec<Base>) {
        for x in self {
            x.to_fields(out);
        }
    }
}

pub trait FromFields {
    const FIELD_COUNT: usize;
    fn from_fields(iter: &mut impl Iterator<Item = Base>) -> Self;
}

impl FromFields for Element {
    const FIELD_COUNT: usize = 1;
    fn from_fields(iter: &mut impl Iterator<Item = Base>) -> Self {
        let base = iter.next().expect("not enough fields to unpack");
        Element::from_base(base)
    }
}

impl<const N: usize, T: FromFields + Debug> FromFields for [T; N] {
    const FIELD_COUNT: usize = N * T::FIELD_COUNT;

    fn from_fields(iter: &mut impl Iterator<Item = Base>) -> Self {
        (0..N)
            .map(|_| T::from_fields(iter))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

// Boolean implementations
impl ToFields for bool {
    fn to_fields(&self, out: &mut Vec<Base>) {
        out.push(Base::from(*self as u64));
    }
}

impl FromFields for bool {
    const FIELD_COUNT: usize = 1;

    fn from_fields(iter: &mut impl Iterator<Item = Base>) -> Self {
        let base = iter.next().expect("not enough fields to unpack");
        !base.is_zero()
    }
}

// Integer implementations
pub trait IntToBase: Sized {
    fn to_base(self) -> Base;
    fn from_base(base: Base) -> Self;
}

macro_rules! impl_int_to_base {
    ($($ty:ty => $unsigned:ty),* $(,)?) => {
        $(
            impl IntToBase for $ty {
                fn to_base(self) -> Base {
                    Base::from(self as $unsigned as u128)
                }

                fn from_base(base: Base) -> Self {
                    (base.to_u128() as $unsigned) as $ty
                }
            }
        )*
    };
}

impl_int_to_base!(
    u8 => u8,
    i8 => u8,
    u16 => u16,
    i16 => u16,
    u32 => u32,
    i32 => u32,
    u64 => u64,
    i64 => u64,
    u128 => u128,
    i128 => u128,
);

macro_rules! impl_to_from_fields_for_int {
    ($($ty:ty),*) => {
        $(
            impl ToFields for $ty {
                fn to_fields(&self, out: &mut Vec<Base>) {
                    out.push(IntToBase::to_base(*self));
                }
            }

            impl FromFields for $ty {
                const FIELD_COUNT: usize = 1;

                fn from_fields(iter: &mut impl Iterator<Item = Base>) -> Self {
                    let base = iter.next().expect("not enough fields to unpack");
                    IntToBase::from_base(base)
                }
            }
        )*
    };
}

impl_to_from_fields_for_int!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128);

// Tuple implementations
macro_rules! impl_to_from_fields_for_tuple {
    ($($idx:tt: $name:ident),*) => {
        impl<$($name: ToFields),*> ToFields for ($($name,)*) {
            fn to_fields(&self, out: &mut Vec<Base>) {
                $(self.$idx.to_fields(out);)*
            }
        }

        impl<$($name: FromFields),*> FromFields for ($($name,)*) {
            const FIELD_COUNT: usize = 0 $( + $name::FIELD_COUNT )*;

            fn from_fields(iter: &mut impl Iterator<Item = Base>) -> Self {
                ($($name::from_fields(iter),)*)
            }
        }
    };
}

impl_to_from_fields_for_tuple!(0: A, 1: B);
impl_to_from_fields_for_tuple!(0: A, 1: B, 2: C);
impl_to_from_fields_for_tuple!(0: A, 1: B, 2: C, 3: D);
impl_to_from_fields_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E);
impl_to_from_fields_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_to_from_fields_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_to_from_fields_for_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
