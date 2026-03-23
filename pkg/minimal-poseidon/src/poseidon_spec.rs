use ff::FromUniformBytes;
use halo2curves::bn256::Fr as Bn256Fr;
use std::marker::PhantomData;

use crate::bn256_constants::{MDS, MDS_INV, ROUND_CONSTANTS};
use crate::core_logic::{CachedSpec, Mds, Spec}; // Actual constants

// --- Paste from poseidon-base/src/primitives/p128pow5t3.rs ---
/// The trait required for fields can handle a pow5 sbox, 3 field, 2 rate permutation
pub trait P128Pow5T3Constants: FromUniformBytes<64> + Ord {
    fn partial_rounds() -> usize {
        56 // Default, overridden by Bn256Fr impl
    }
    fn round_constants() -> Vec<[Self; 3]>;
    fn mds() -> Mds<Self, 3>;
    fn mds_inv() -> Mds<Self, 3>;
}

/// Poseidon-128 using the $x^5$ S-box, with a width of 3 field elements, and the
/// standard number of rounds for 128-bit security "with margin".
#[derive(Debug, Copy, Clone)]
pub struct P128Pow5T3<C> {
    _marker: PhantomData<C>,
}

impl<Fp: P128Pow5T3Constants> Spec<Fp, 3, 2> for P128Pow5T3<Fp> {
    fn full_rounds() -> usize {
        8
    }

    fn partial_rounds() -> usize {
        Fp::partial_rounds()
    }

    fn sbox(val: Fp) -> Fp {
        val.pow_vartime([5])
    }

    fn secure_mds() -> usize {
        // This is not used because constants() is overridden
        unimplemented!(
            "secure_mds is not needed when constants are hardcoded via P128Pow5T3Constants"
        )
    }

    fn constants() -> (Vec<[Fp; 3]>, Mds<Fp, 3>, Mds<Fp, 3>) {
        (Fp::round_constants(), Fp::mds(), Fp::mds_inv())
    }
}
// --- End of paste ---

// CachedSpec impl for P128Pow5T3<Bn256Fr>
impl CachedSpec<Bn256Fr, 3, 2> for P128Pow5T3<Bn256Fr> {
    fn cached_round_constants() -> &'static [[Bn256Fr; 3]] {
        &*ROUND_CONSTANTS
    }
    fn cached_mds() -> &'static Mds<Bn256Fr, 3> {
        &MDS
    }
    fn cached_mds_inv() -> &'static Mds<Bn256Fr, 3> {
        &MDS_INV
    }
}
