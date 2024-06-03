//! Decomposes an $n$-bit field element $\alpha$ into $W$ windows, each window
//! being a $K$-bit word, using a running sum $z$.
//! We constrain $K \leq 3$ for this helper.
//!     $$\alpha = k_0 + (2^K) k_1 + (2^{2K}) k_2 + ... + (2^{(W-1)K}) k_{W-1}$$
//!
//! $z_0$ is initialized as $\alpha$. Each successive $z_{i+1}$ is computed as
//!                $$z_{i+1} = (z_{i} - k_i) / (2^K).$$
//! $z_W$ is constrained to be zero.
//! The difference between each interstitial running sum output is constrained
//! to be $K$ bits, i.e.
//!                      `range_check`($k_i$, $2^K$),
//! where
//! ```text
//!   range_check(word, range)
//!     = word * (1 - word) * (2 - word) * ... * ((range - 1) - word)
//! ```
//!
//! Given that the `range_check` constraint will be toggled by a selector, in
//! practice we will have a `selector * range_check(word, range)` expression
//! of degree `range + 1`.
//!
//! This means that $2^K$ has to be at most `degree_bound - 1` in order for
//! the range check constraint to stay within the degree bound.

// use ff::PrimeFieldBits;
use halo2_base::halo2_proofs::{
    circuit::{AssignedCell, Region, Value},
    halo2curves::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Constraints, Error, Expression, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

use crate::fr::PrimeFieldBits;

/// Check that an expression is in the small range [0..range),
/// i.e. 0 ≤ word < range.
pub fn range_check<F: FieldExt>(word: Expression<F>, range: usize) -> Expression<F> {
    (1..range).fold(word.clone(), |acc, i| {
        acc * (Expression::Constant(F::from(i as u64)) - word.clone())
    })
}

pub fn decompose_word<F: PrimeFieldBits>(
    word: &F,
    word_num_bits: usize,
    window_num_bits: usize,
) -> Vec<u8> {
    assert!(window_num_bits <= 8);

    // Pad bits to multiple of window_num_bits
    let padding = (window_num_bits - (word_num_bits % window_num_bits)) % window_num_bits;
    let bits: Vec<bool> = word
        .to_le_bits()
        .into_iter()
        .take(word_num_bits)
        .chain(std::iter::repeat(false).take(padding))
        .collect();
    assert_eq!(bits.len(), word_num_bits + padding);

    bits.chunks_exact(window_num_bits)
        .map(|chunk| chunk.iter().rev().fold(0, |acc, b| (acc << 1) + (*b as u8)))
        .collect()
}

/// The running sum $[z_0, ..., z_W]$. If created in strict mode, $z_W = 0$.
#[derive(Debug)]
pub struct BinaryDecomposition<F: FieldExt + PrimeFieldBits>(pub(crate) Vec<AssignedCell<F, F>>);
impl<F: FieldExt + PrimeFieldBits> std::ops::Deref for BinaryDecomposition<F> {
    type Target = Vec<AssignedCell<F, F>>;

    fn deref(&self) -> &Vec<AssignedCell<F, F>> {
        &self.0
    }
}

/// Configuration that provides methods for running sum decomposition.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BinaryDecompositionConfig<F: FieldExt + PrimeFieldBits, const WINDOW_NUM_BITS: usize> {
    q_range_check: Selector,
    z: Column<Advice>,
    b: Column<Advice>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt + PrimeFieldBits, const WINDOW_NUM_BITS: usize>
    BinaryDecompositionConfig<F, WINDOW_NUM_BITS>
{
    /// `perm` MUST include the advice column `z`.
    ///
    /// # Panics
    ///
    /// Panics if WINDOW_NUM_BITS > 3.
    ///
    /// # Side-effects
    ///
    /// `z` will be equality-enabled.
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        q_range_check: Selector,
        z: Column<Advice>,
        b: Column<Advice>,
    ) -> Self {
        assert!(WINDOW_NUM_BITS <= 3);

        meta.enable_equality(z);

        let config = Self {
            q_range_check,
            z,
            b,
            _marker: PhantomData,
        };

        // https://p.z.cash/halo2-0.1:decompose-short-range
        meta.create_gate("range check", |meta| {
            let q_range_check = meta.query_selector(config.q_range_check);
            let z_cur = meta.query_advice(config.z, Rotation::cur());
            let z_next = meta.query_advice(config.z, Rotation::next());
            let b = meta.query_advice(b, Rotation::cur());
            //    z_i = 2^{K}⋅z_{i + 1} + k_i
            // => k_i = z_i - 2^{K}⋅z_{i + 1}
            let word = z_cur - z_next * F::from(1 << WINDOW_NUM_BITS);

            Constraints::with_selector(
                q_range_check,
                [range_check(word.clone(), 1 << WINDOW_NUM_BITS), b - word],
            )
        });

        config
    }

    /// Decompose a field element alpha that is witnessed in this helper.
    ///
    /// `strict` = true constrains the final running sum to be zero, i.e.
    /// constrains alpha to be within WINDOW_NUM_BITS * num_windows bits.
    pub fn witness_decompose(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        alpha: Value<F>,
        word_num_bits: usize,
        num_windows: usize,
    ) -> Result<BinaryDecomposition<F>, Error> {
        let z_0 = region.assign_advice(|| "z_0 = alpha", self.z, offset, || alpha)?;
        self.decompose(region, offset, z_0, word_num_bits, num_windows)
    }

    /// Decompose an existing variable alpha that is copied into this helper.
    ///
    /// `strict` = true constrains the final running sum to be zero, i.e.
    /// constrains alpha to be within WINDOW_NUM_BITS * num_windows bits.
    pub fn copy_decompose(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        alpha: AssignedCell<F, F>,
        word_num_bits: usize,
        num_windows: usize,
    ) -> Result<BinaryDecomposition<F>, Error> {
        let z_0 = alpha.copy_advice(|| "copy z_0 = alpha", region, self.z, offset)?;
        self.decompose(region, offset, z_0, word_num_bits, num_windows)
    }

    /// `z_0` must be the cell at `(self.z, offset)` in `region`.
    ///
    /// # Panics
    ///
    /// Panics if there are too many windows for the given word size.
    fn decompose(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        z_0: AssignedCell<F, F>,
        word_num_bits: usize,
        num_windows: usize,
    ) -> Result<BinaryDecomposition<F>, Error> {
        // Make sure that we do not have more windows than required for the number
        // of bits in the word. In other words, every window must contain at least
        // one bit of the word (no empty windows).
        //
        // For example, let:
        //      - word_num_bits = 64
        //      - WINDOW_NUM_BITS = 3
        // In this case, the maximum allowed num_windows is 22:
        //                    3 * 22 < 64 + 3
        //
        assert!(WINDOW_NUM_BITS * num_windows < word_num_bits + WINDOW_NUM_BITS);

        // Enable selectors
        for idx in 0..num_windows {
            self.q_range_check.enable(region, offset + idx)?;
        }

        // Decompose base field element into K-bit words.
        let words = z_0
            .value()
            .map(|word| decompose_word::<F>(word, word_num_bits, WINDOW_NUM_BITS))
            .transpose_vec(num_windows);

        // Initialize empty vector to store running sum values [z_0, ..., z_W].
        let mut zs: Vec<AssignedCell<F, F>> = vec![];
        let mut z = z_0;

        // Assign padding to the first element?
        // region.assign_advice(
        //     || format!("b_{:?}", 0),
        //     self.b,
        //     offset,
        //     || Value::known(F::zero()),
        // )?;

        // Assign running sum `z_{i+1}` = (z_i - k_i) / (2^K) for i = 0..=n-1.
        // Outside of this helper, z_0 = alpha must have already been loaded into the
        // `z` column at `offset`.
        let two_pow_k_inv = Value::known(F::from(1 << WINDOW_NUM_BITS as u64).invert().unwrap());
        for (i, word) in words.iter().enumerate() {
            // z_next = (z_cur - word) / (2^K)

            let word = word.map(|word| F::from(word as u64));

            let z_next = {
                let z_cur_val = z.value().copied();
                let z_next_val = (z_cur_val - word) * two_pow_k_inv;
                region.assign_advice(
                    || format!("z_{:?}", i + 1),
                    self.z,
                    offset + i + 1,
                    || z_next_val,
                )?
            };

            let b_assigned =
                region.assign_advice(|| format!("b_{i:?}"), self.b, offset + i, || word)?;

            // Update `z`.
            z = z_next;
            zs.push(b_assigned);
        }
        assert_eq!(zs.len(), num_windows);

        // Constrain the final running sum output to be zero.
        region.constrain_constant(z.cell(), F::zero())?;

        Ok(BinaryDecomposition(zs))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::util::{advice_column_equality, instance_column_equality};

    use super::*;
    use halo2_base::halo2_proofs::plonk::Instance;
    use halo2_base::halo2_proofs::{
        circuit::{Layouter, SimpleFloorPlanner},
        dev::MockProver,
        halo2curves::{bn256::Fr, FieldExt},
        plonk::{Circuit, ConstraintSystem, Error},
    };

    struct BinaryDecompCircuit<F: FieldExt + PrimeFieldBits, const WINDOW_NUM_BITS: usize> {
        alpha: Value<F>,
        word_num_bits: usize,
        num_windows: usize,
    }

    impl<F: FieldExt + PrimeFieldBits, const WINDOW_NUM_BITS: usize> Circuit<F>
        for BinaryDecompCircuit<F, WINDOW_NUM_BITS>
    {
        type Config = (
            Column<Instance>,
            BinaryDecompositionConfig<F, WINDOW_NUM_BITS>,
        );
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self {
                alpha: Value::unknown(),
                word_num_bits: self.word_num_bits,
                num_windows: self.num_windows,
            }
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            let z = advice_column_equality(meta);
            let b = advice_column_equality(meta);
            let i = instance_column_equality(meta);
            let q_range_check = meta.selector();
            let constants = meta.fixed_column();
            meta.enable_constant(constants);

            (
                i,
                BinaryDecompositionConfig::<F, WINDOW_NUM_BITS>::configure(
                    meta,
                    q_range_check,
                    z,
                    b,
                ),
            )
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            let (instance, config) = config;
            let bits = layouter.assign_region(
                || "decompose",
                |mut region| {
                    let offset = 0;
                    config.witness_decompose(
                        &mut region,
                        offset,
                        self.alpha,
                        self.word_num_bits,
                        self.num_windows,
                    )
                },
            )?;

            for (i, b) in bits.iter().enumerate() {
                layouter.constrain_instance(b.cell(), instance, i)?;
            }

            Ok(())
        }
    }

    #[test]
    fn test_binary_decomp() {
        let k = 14;

        let circuit = BinaryDecompCircuit::<Fr, 1> {
            alpha: Value::known(Fr::from(3)),
            num_windows: 3,
            word_num_bits: 3,
        };

        // 011 -> 1,1,0
        let bits = vec![Fr::one(), Fr::one(), Fr::zero()];

        let prover = MockProver::<Fr>::run(k, &circuit, vec![bits]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_binary_decomp_2() {
        let k = 14;

        let circuit = BinaryDecompCircuit::<Fr, 1> {
            alpha: Value::known(Fr::from(7)),
            num_windows: 3,
            word_num_bits: 3,
        };

        // 111 -> 1,1,1
        let bits = vec![Fr::one(), Fr::one(), Fr::one()];

        let prover = MockProver::<Fr>::run(k, &circuit, vec![bits]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_binary_decomp_fail() {
        let k = 14;

        let circuit = BinaryDecompCircuit::<Fr, 1> {
            alpha: Value::known(Fr::from(7)),
            num_windows: 3,
            word_num_bits: 3,
        };

        // 111 -> 1,1,1
        let bits = vec![Fr::one(), Fr::one(), Fr::zero()];

        let prover = MockProver::<Fr>::run(k, &circuit, vec![bits]).unwrap();
        prover.verify().unwrap_err();
    }
}
