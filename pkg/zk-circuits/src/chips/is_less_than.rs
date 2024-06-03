use halo2_base::halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct IsLessThanChipConfig {
    selector: Selector,
    advices: [Column<Advice>; 4],
}

/// This chip should be used to compare two binary values (with each bit being represented by an assigned cell).
/// It is assumed that the bit values have been range checked, and are in big-endian order.
#[derive(Debug, Clone)]
pub struct IsLessThanChip<F: Field> {
    config: IsLessThanChipConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> IsLessThanChip<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advices: [Column<Advice>; 4],
    ) -> IsLessThanChipConfig {
        let selector = meta.selector();

        let max = advices[0];
        let alpha = advices[1];
        let is_less = advices[2];
        let can_set = advices[3];

        meta.create_gate("Is Less", |meta| {
            let s = meta.query_selector(selector);

            let last_is_less = meta.query_advice(is_less, Rotation::prev());
            let last_can_set = meta.query_advice(can_set, Rotation::prev());

            let max = meta.query_advice(max, Rotation::cur());
            let alpha = meta.query_advice(alpha, Rotation::cur());
            let is_less = meta.query_advice(is_less, Rotation::cur());
            let can_set = meta.query_advice(can_set, Rotation::cur());

            let one = Expression::Constant(F::one());

            //  max   |  alpha   |  is_less         |  can_set       |  selector
            //                   |  last_is_less    |  last_can_set  |
            //  max   |  alpha   |  is_less         |  can_set       |  1

            vec![
                // Constraint is_less
                s.clone()
                    * ((last_is_less
                        - (max.clone() * (one.clone() - alpha.clone())) * last_can_set.clone())
                        - is_less),
                // Constrain can_set
                s * ((max.clone() * alpha.clone() + (one.clone() - max) * (one - alpha))
                    * last_can_set
                    - can_set),
            ]
        });

        IsLessThanChipConfig { selector, advices }
    }

    pub fn construct(config: IsLessThanChipConfig) -> Self {
        Self {
            _marker: PhantomData,
            config,
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        max_bits: &[AssignedCell<F, F>],
        alpha_bits: &[AssignedCell<F, F>],
    ) -> Result<(), Error> {
        assert_eq!(
            max_bits.len(),
            alpha_bits.len(),
            "max and alpha bits must have the same length"
        );

        layouter.assign_region(
            || "is less than chip",
            |mut region| {
                let advices = self.config.advices;

                let max_advice = advices[0];
                let alpha_advice = advices[1];
                let is_less_advice = advices[2];
                let can_set_advice = advices[3];

                // Copy the first region values
                let mut is_less = region.assign_advice_from_constant(
                    || "init is_less",
                    is_less_advice,
                    0,
                    F::one(),
                )?;
                let mut can_set = region.assign_advice_from_constant(
                    || "init can_set",
                    can_set_advice,
                    0,
                    F::one(),
                )?;

                for (i, (m_bit, a_bit)) in max_bits.iter().zip(alpha_bits.iter()).enumerate() {
                    let offset = i + 1;

                    // Enable the selector, so we constrain all rows (except the first)
                    self.config.selector.enable(&mut region, offset)?;

                    // Copy in the values to be compared
                    let m_bit = m_bit.copy_advice(|| "m_bit", &mut region, max_advice, offset)?;
                    let a_bit = a_bit.copy_advice(|| "a_bit", &mut region, alpha_advice, offset)?;

                    // is_less expected value
                    is_less = region.assign_advice(
                        || "is less",
                        is_less_advice,
                        offset,
                        || {
                            is_less.value().and_then(|last_is_less| {
                                can_set.value().and_then(|last_can_set| {
                                    m_bit.value().and_then(|m_val| {
                                        a_bit.value().and_then(|a_val| {
                                            Value::known(
                                                *last_is_less
                                                    - (*m_val
                                                        * (F::one() - *a_val)
                                                        * *last_can_set),
                                            )
                                        })
                                    })
                                })
                            })
                        },
                    )?;

                    // can_set expected value
                    can_set = region.assign_advice(
                        || "can_set",
                        can_set_advice,
                        offset,
                        || {
                            can_set.value().and_then(|last_can_set| {
                                m_bit.value().and_then(|m_val| {
                                    a_bit.value().and_then(|a_val| {
                                        Value::known(
                                            (*m_val * *a_val
                                                + (F::one() - *m_val) * (F::one() - *a_val))
                                                * *last_can_set,
                                        )
                                    })
                                })
                            })
                        },
                    )?;
                }

                // Assert the final value is zero
                region.constrain_constant(is_less.cell(), F::zero())?;

                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    // use halo2_base::halo2_proofs::arithmetic::FieldExt;
    use halo2_base::halo2_proofs::{
        circuit::SimpleFloorPlanner,
        dev::MockProver,
        halo2curves::bn256::Fr,
        plonk::{Circuit, Error},
    };

    use crate::{test::util::advice_column_equality, util::assign_private_input};

    use super::*;

    #[derive(Default, Clone, Debug)]
    struct IsLessThanCircuit {
        max: Vec<Fr>,
        alpha: Vec<Fr>,
    }

    impl Circuit<Fr> for IsLessThanCircuit {
        type Config = IsLessThanChipConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            let advices = [
                advice_column_equality(meta),
                advice_column_equality(meta),
                advice_column_equality(meta),
                advice_column_equality(meta),
            ];
            let fixed = meta.fixed_column();
            meta.enable_constant(fixed);
            IsLessThanChip::configure(meta, advices)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let is_less_than_chip = IsLessThanChip::construct(config.clone());

            let max_bits = self
                .max
                .iter()
                .map(|v| {
                    assign_private_input(
                        || "max",
                        layouter.namespace(|| "max"),
                        config.advices[0],
                        Value::known(*v),
                    )
                })
                .collect::<Result<Vec<_>, Error>>()?;

            let alpha_bits = self
                .alpha
                .iter()
                .map(|v| {
                    assign_private_input(
                        || "max",
                        layouter.namespace(|| "alpha"),
                        config.advices[1],
                        Value::known(*v),
                    )
                })
                .collect::<Result<Vec<_>, Error>>()?;

            is_less_than_chip.assign(
                layouter.namespace(|| "is less than"),
                max_bits.as_slice(),
                alpha_bits.as_slice(),
            )?;

            Ok(())
        }
    }

    #[test]
    fn test_less_than_1_bit() {
        let k = 8;

        let circuit = IsLessThanCircuit {
            max: vec![Fr::from(1u64)],
            alpha: vec![Fr::from(0u64)],
        };

        let prover = MockProver::<Fr>::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_not_less_than_1_bit_error() {
        let k = 8;

        let circuit = IsLessThanCircuit {
            max: vec![Fr::from(1u64)],
            alpha: vec![Fr::from(1u64)],
        };

        let prover = MockProver::<Fr>::run(k, &circuit, vec![]).unwrap();
        prover.verify().expect_err("proof should not be satisfied");
    }

    #[test]
    fn test_less_than_6_bits() {
        let k = 8;

        let circuit = IsLessThanCircuit {
            max: vec![
                Fr::from(0u64),
                Fr::from(1u64),
                Fr::from(0u64),
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(0u64),
            ],
            alpha: vec![
                Fr::from(0u64),
                Fr::from(0u64),
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(1u64),
            ],
        };

        let prover = MockProver::<Fr>::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_less_than_6_bits_error() {
        let k = 8;

        let circuit = IsLessThanCircuit {
            max: vec![
                Fr::from(0u64),
                Fr::from(1u64),
                Fr::from(0u64),
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(0u64),
            ],
            alpha: vec![
                Fr::from(0u64),
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(0u64),
                Fr::from(0u64),
                Fr::from(0u64),
            ],
        };

        let prover = MockProver::<Fr>::run(k, &circuit, vec![]).unwrap();
        prover.verify().expect_err("proof should not be satisfied");
    }
}
