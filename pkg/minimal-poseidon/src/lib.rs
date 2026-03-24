pub use halo2curves::bn256::Fr as Bn256Fr; // Or your preferred alias for the base field

pub use crate::core_logic::{ConstantLength, Hash};
pub use crate::poseidon_spec::{P128Pow5T3, P128Pow5T3Constants};

mod bn256_constants;
mod core_logic;
mod poseidon_spec;
