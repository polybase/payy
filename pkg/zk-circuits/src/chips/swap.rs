//! Gadget and chip for a conditional swap utility.
use halo2_base::halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Region, Value},
    plonk::{Advice, Column, ConstraintSystem, Constraints, Error, Selector},
    poly::Rotation,
};
use halo2_gadgets::utilities::{bool_check, ternary};
use std::marker::PhantomData;

/// A chip implementing a conditional swap.
#[derive(Clone, Debug)]
pub struct CondSwapChip<F: FieldExt> {
    config: CondSwapConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> CondSwapChip<F> {
    #[allow(clippy::type_complexity)]
    pub fn swap(
        &self,
        mut layouter: impl Layouter<F>,
        pair: (&AssignedCell<F, F>, Value<F>),
        swap: Value<F>,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        let config = &self.config;
        layouter.assign_region(
            || "swap",
            |mut region| {
                // Copy in `a` value
                let a = pair.0.copy_advice(|| "copy a", &mut region, config.a, 0)?;

                // Witness `b` value
                let b = region.assign_advice(|| "witness b", config.b, 0, || pair.1)?;

                // Witness `swap` value
                let swap = region.assign_advice(|| "swap", config.swap, 0, || swap)?;

                self.swap_in_region(region, a, b, swap)
            },
        )
    }

    #[allow(clippy::type_complexity)]
    pub fn swap_assigned(
        &self,
        mut layouter: impl Layouter<F>,
        pair: (&AssignedCell<F, F>, &AssignedCell<F, F>),
        swap: &AssignedCell<F, F>,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        let config = &self.config;
        layouter.assign_region(
            || "swap assigned",
            |mut region| {
                // Copy in `a` value
                let a = pair.0.copy_advice(|| "copy a", &mut region, config.a, 0)?;

                // Copy in `b` value
                let b = pair.1.copy_advice(|| "copy b", &mut region, config.b, 0)?;

                // Witness `swap` value
                let swap = swap.copy_advice(|| "swap value", &mut region, config.swap, 0)?;

                self.swap_in_region(region, a, b, swap)
            },
        )
    }

    #[allow(clippy::type_complexity)]
    pub fn swap_in_region(
        &self,
        mut region: Region<F>,
        a: AssignedCell<F, F>,
        b: AssignedCell<F, F>,
        swap: AssignedCell<F, F>,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        let config = &self.config;
        // Enable `q_swap` selector
        config.q_swap.enable(&mut region, 0)?;

        // Conditionally swap a
        let a_swapped = {
            let a_swapped = a
                .value()
                .zip(b.value())
                .zip(swap.value())
                .map(|((a, b), swap)| if *swap == F::one() { b } else { a })
                .cloned();
            region.assign_advice(|| "a_swapped", config.a_swapped, 0, || a_swapped)?
        };

        // Conditionally swap b
        let b_swapped = {
            let b_swapped = a
                .value()
                .zip(b.value())
                .zip(swap.value())
                .map(|((a, b), swap)| if *swap == F::one() { a } else { b })
                .cloned();
            region.assign_advice(|| "b_swapped", config.b_swapped, 0, || b_swapped)?
        };

        // Return swapped pair
        Ok((a_swapped, b_swapped))
    }
}

/// Configuration for the [`CondSwapChip`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CondSwapConfig {
    q_swap: Selector,
    a: Column<Advice>,
    b: Column<Advice>,
    a_swapped: Column<Advice>,
    b_swapped: Column<Advice>,
    swap: Column<Advice>,
}

impl<F: FieldExt> CondSwapChip<F> {
    /// Configures this chip for use in a circuit.
    ///
    /// # Side-effects
    ///
    /// `advices[0]` will be equality-enabled.
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advices: [Column<Advice>; 5],
    ) -> CondSwapConfig {
        let q_swap = meta.selector();

        let config = CondSwapConfig {
            q_swap,
            a: advices[0],
            b: advices[1],
            a_swapped: advices[2],
            b_swapped: advices[3],
            swap: advices[4],
        };

        meta.enable_equality(config.a);
        meta.enable_equality(config.b);
        meta.enable_equality(config.swap);

        // TODO: optimise shape of gate for Merkle path validation

        meta.create_gate("a' = b ⋅ swap + a ⋅ (1-swap)", |meta| {
            let q_swap = meta.query_selector(q_swap);

            let a = meta.query_advice(config.a, Rotation::cur());
            let b = meta.query_advice(config.b, Rotation::cur());
            let a_swapped = meta.query_advice(config.a_swapped, Rotation::cur());
            let b_swapped = meta.query_advice(config.b_swapped, Rotation::cur());
            let swap = meta.query_advice(config.swap, Rotation::cur());

            // This checks that `a_swapped` is equal to `b` when `swap` is set,
            // but remains as `a` when `swap` is not set.
            let a_check = a_swapped - ternary(swap.clone(), b.clone(), a.clone());

            // This checks that `b_swapped` is equal to `a` when `swap` is set,
            // but remains as `b` when `swap` is not set.
            let b_check = b_swapped - ternary(swap.clone(), a, b);

            // Check `swap` is boolean.
            let bool_check = bool_check(swap);

            Constraints::with_selector(
                q_swap,
                [
                    ("a check", a_check),
                    ("b check", b_check),
                    ("swap is bool", bool_check),
                ],
            )
        });

        config
    }

    /// Constructs a [`CondSwapChip`] given a [`CondSwapConfig`].
    pub fn construct(config: CondSwapConfig) -> Self {
        CondSwapChip {
            config,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::assign_private_input;

    use super::{CondSwapChip, CondSwapConfig};
    use halo2_base::halo2_proofs::{
        arithmetic::Field,
        circuit::{Layouter, SimpleFloorPlanner, Value},
        dev::MockProver,
        halo2curves::{bn256::Fr, FieldExt},
        plonk::{Circuit, ConstraintSystem, Error},
    };
    use rand::rngs::OsRng;

    #[test]
    fn cond_swap() {
        #[derive(Default)]
        struct MyCircuit<F: FieldExt> {
            a: Value<F>,
            b: Value<F>,
            swap: Value<F>,
        }

        impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
            type Config = CondSwapConfig;
            type FloorPlanner = SimpleFloorPlanner;

            fn without_witnesses(&self) -> Self {
                Self::default()
            }

            fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
                let advices = [
                    meta.advice_column(),
                    meta.advice_column(),
                    meta.advice_column(),
                    meta.advice_column(),
                    meta.advice_column(),
                ];

                CondSwapChip::<F>::configure(meta, advices)
            }

            fn synthesize(
                &self,
                config: Self::Config,
                mut layouter: impl Layouter<F>,
            ) -> Result<(), Error> {
                let chip = CondSwapChip::<F>::construct(config.clone());

                // Load the pair and the swap flag into the circuit.
                // let a = chip.load_private(layouter.namespace(|| "a"), config.a, self.a)?;
                let a = assign_private_input(
                    || "assign a",
                    layouter.namespace(|| "assign a"),
                    config.a,
                    self.a,
                )?;
                let b = assign_private_input(
                    || "assign b",
                    layouter.namespace(|| "assign b"),
                    config.b,
                    self.b,
                )?;
                let swap = assign_private_input(
                    || "assign swap",
                    layouter.namespace(|| "assign swap"),
                    config.a,
                    self.swap,
                )?;

                // Return the swapped pair.
                let swapped_pair =
                    chip.swap_assigned(layouter.namespace(|| "swap"), (&a, &b), &swap)?;

                self.swap
                    .zip(a.value().zip(self.b.as_ref()))
                    .zip(swapped_pair.0.value().zip(swapped_pair.1.value()))
                    .assert_if_known(|((swap, (a, b)), (a_swapped, b_swapped))| {
                        if *swap == F::one() {
                            // Check that `a` and `b` have been swapped
                            (a_swapped == b) && (b_swapped == a)
                        } else {
                            // Check that `a` and `b` have not been swapped
                            (a_swapped == a) && (b_swapped == b)
                        }
                    });

                Ok(())
            }
        }

        let rng = OsRng;

        // Test swap case
        {
            let circuit: MyCircuit<Fr> = MyCircuit {
                a: Value::known(Fr::random(rng)),
                b: Value::known(Fr::random(rng)),
                swap: Value::known(Fr::one()),
            };
            let prover = MockProver::<Fr>::run(4, &circuit, vec![]).unwrap();
            assert_eq!(prover.verify(), Ok(()));
        }

        // Test non-swap case
        {
            let circuit: MyCircuit<Fr> = MyCircuit {
                a: Value::known(Fr::random(rng)),
                b: Value::known(Fr::random(rng)),
                swap: Value::known(Fr::zero()),
            };
            let prover = MockProver::<Fr>::run(4, &circuit, vec![]).unwrap();
            assert_eq!(prover.verify(), Ok(()));
        }
    }
}
