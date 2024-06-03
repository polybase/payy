use halo2_base::halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Region, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
    poly::Rotation,
};

#[derive(Clone, Debug)]
pub struct IsZeroConfig<F> {
    pub value_inv: Column<Advice>,
    pub is_zero_expr: Expression<F>,
}

/// IsZeroChip
///
/// Cannot be used standalone, you MUST use the configured isZeroChip with another custom gate
/// to actually use the isZeroExpr value which will be 1 if the value is zero, and 0 otherwise.
///
#[derive(Clone, Debug)]
pub struct IsZeroChip<F: FieldExt> {
    config: IsZeroConfig<F>,
}

impl<F: FieldExt> IsZeroChip<F> {
    pub fn construct(config: IsZeroConfig<F>) -> Self {
        IsZeroChip { config }
    }

    // Here we deal with expressions that must evaluate to 0, these constrain the assignment values of a circuit
    //  (in the next step)
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        // Selector
        q_enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        // Value expression to check for zero (can including multiple columns)
        value: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        value_inv: Column<Advice>,
    ) -> IsZeroConfig<F> {
        let mut is_zero_expr = Expression::Constant(F::zero());

        // GATE WILL EVALUATE TO 1 if ZERO, 0 otherwise!
        meta.create_gate("is_zero", |meta| {
            //
            // valid | value |  value_inv |  1 - value * value_inv | value * (1 - value* value_inv)
            // ------+-------+------------+------------------------+-------------------------------
            //  yes  |   x   |    1/x     |         0              |  0
            //  no   |   x   |    0       |         1              |  x
            //  yes  |   0   |    0       |         1              |  0
            //  yes  |   0   |    y       |         1              |  0
            //
            let value = value(meta);
            let q_enable = q_enable(meta);
            let value_inv = meta.query_advice(value_inv, Rotation::cur());

            // This will be used inside another constrait gate!
            is_zero_expr = Expression::Constant(F::one()) - value.clone() * value_inv;

            // This is an additional constraint check to prevent the value_inv being populated incorrectly! This is populated
            // by the prover, in assign below, so we must ensure they have used a valid value!
            vec![q_enable * value * is_zero_expr.clone()]
        });

        IsZeroConfig {
            value_inv,

            // is_zero_expr will evalautate to 1 if the value is 0, and 1 otherwise. That means we can basically
            // think of is_zero_expr as a selector.

            // If we want to check if an expr evaluates to 0, we can use is_zero_expr * expr, because if expr is != 0
            // then selector will cause the value to not be 0.

            // If we want to check if an expr evaluates to 1 (i.e. not zero), we can use (1 - is_zero_expr) * expr
            is_zero_expr,
        }
    }

    pub fn assign(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        value: Value<F>,
    ) -> Result<(), Error> {
        let value_inv = value.map(|value| value.invert().unwrap_or(F::zero()));
        region.assign_advice(|| "value inv", self.config.value_inv, offset, || value_inv)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::util::{advice_column_equality, instance_column_equality};

    use super::*;
    use halo2_base::halo2_proofs::{
        arithmetic::FieldExt,
        circuit::SimpleFloorPlanner,
        dev::{FailureLocation, MockProver, VerifyFailure},
        halo2curves::pasta::Fp,
        plonk::{Advice, Any, Circuit, Column, ConstraintSystem, Constraints, Instance, Selector},
        poly::Rotation,
    };

    /////////
    ///
    /// Impl chip in circuit
    ///
    ////////

    #[derive(Debug, Clone)]
    struct TestCircuitConfig<F: FieldExt> {
        zero_chip_config: IsZeroConfig<F>,
        value_to_check_advice: Column<Advice>,
        bool_is_zero_advice: Column<Advice>,
        instance: Column<Instance>,
        selector: Selector,
    }

    #[derive(Debug, Default)]
    struct TestCircuit<F: FieldExt> {
        is_zero: F,
    }

    impl<F: FieldExt> Circuit<F> for TestCircuit<F> {
        type Config = TestCircuitConfig<F>;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        // Constrain the circuit
        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            let value_to_check_advice: Column<Advice> = advice_column_equality(meta);
            let bool_is_zero_advice = advice_column_equality(meta);
            let zero_col = advice_column_equality(meta);
            let instance_col = instance_column_equality(meta);
            let selector: Selector = meta.selector();

            // Configure the isZero chip so we can get access to the
            let is_zero_configure = IsZeroChip::<F>::configure(
                meta,
                |meta| meta.query_selector(selector),
                // This is the value being checked
                |meta| meta.query_advice(value_to_check_advice, Rotation::cur()),
                // This is the inverse of value bieng checked  (required for safety)
                zero_col,
            );

            meta.create_gate("is_zero", |meta| {
                let s = meta.query_selector(selector);
                let bool_is_zero = meta.query_advice(bool_is_zero_advice, Rotation::cur());

                Constraints::with_selector(
                    s,
                    [bool_is_zero - is_zero_configure.is_zero_expr.clone()],
                )
            });

            TestCircuitConfig {
                zero_chip_config: is_zero_configure,
                value_to_check_advice,
                bool_is_zero_advice,
                instance: instance_col,
                selector,
            }
        }

        // Populate the circuit
        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl halo2_base::halo2_proofs::circuit::Layouter<F>,
        ) -> Result<(), halo2_base::halo2_proofs::plonk::Error> {
            let zero_chip = IsZeroChip::construct(config.zero_chip_config);

            layouter.assign_region(
                || "check_num",
                |mut region| {
                    // Enables the enforcement of the gate on the pre-configured instance column
                    config.selector.enable(&mut region, 0)?;

                    let cell = region.assign_advice_from_instance(
                        || "value to check is 0",
                        config.instance,
                        0,
                        config.value_to_check_advice,
                        0,
                    )?;

                    // Assign the value to check
                    zero_chip.assign(&mut region, 0, cell.value().copied())?;

                    // Bool is zero
                    region.assign_advice(
                        || "bool is zero",
                        config.bool_is_zero_advice,
                        0,
                        || Value::known(self.is_zero),
                    )?;

                    Ok(())
                },
            )
        }
    }

    #[test]
    fn test_zero() {
        let k = 4;

        let val = Fp::zero();

        let circuit = TestCircuit { is_zero: Fp::one() };

        // Vector for the public input column (if we had more, we'd need to add additional)
        let public_input = vec![val];
        let instance_columns = vec![public_input];

        let prover = MockProver::<Fp>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_zero_invalid() {
        let k = 4;

        let val = Fp::zero();

        let circuit = TestCircuit {
            is_zero: Fp::zero(),
        };

        // Vector for the public input column (if we had more, we'd need to add additional)
        let public_input = vec![val];
        let instance_columns = vec![public_input];

        let prover = MockProver::<Fp>::run(k, &circuit, instance_columns).unwrap();
        assert_eq!(
            prover.verify(),
            Err(vec![VerifyFailure::ConstraintNotSatisfied {
                constraint: ((1, "is_zero").into(), 0, "").into(),
                location: FailureLocation::InRegion {
                    region: (0, "check_num").into(),
                    offset: 0
                },
                cell_values: vec![(
                    ((Any::Advice(Advice::default()), 1).into(), 0).into(),
                    "0".to_string()
                )]
            }])
        );
    }

    #[test]
    fn test_non_zero() {
        let k = 4;

        let val = Fp::from_u128(10u128);

        let circuit = TestCircuit {
            is_zero: Fp::zero(),
        };

        let public_input = vec![val];
        let instance_columns = vec![public_input];

        let prover = MockProver::<Fp>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_non_zero_invalid() {
        let k = 4;

        let val = Fp::from_u128(10u128);

        let circuit = TestCircuit { is_zero: Fp::one() };

        // Vector for the public input column (if we had more, we'd need to add additional)
        let public_input = vec![val];
        let instance_columns = vec![public_input];

        let prover = MockProver::<Fp>::run(k, &circuit, instance_columns).unwrap();
        assert_eq!(
            prover.verify(),
            Err(vec![VerifyFailure::ConstraintNotSatisfied {
                constraint: ((1, "is_zero").into(), 0, "").into(),
                location: FailureLocation::InRegion {
                    region: (0, "check_num").into(),
                    offset: 0
                },
                cell_values: vec![(
                    ((Any::Advice(Advice::default()), 1).into(), 0).into(),
                    "1".to_string()
                )]
            }])
        );
    }

    #[test]
    fn test_non_zero_invalid_non_bool() {
        let k = 4;

        let val = Fp::from_u128(10u128);

        let circuit = TestCircuit {
            is_zero: Fp::from_u128(10u128),
        };

        // Vector for the public input column (if we had more, we'd need to add additional)
        let public_input = vec![val];
        let instance_columns = vec![public_input];

        let prover = MockProver::<Fp>::run(k, &circuit, instance_columns).unwrap();
        assert_eq!(
            prover.verify(),
            Err(vec![VerifyFailure::ConstraintNotSatisfied {
                constraint: ((1, "is_zero").into(), 0, "").into(),
                location: FailureLocation::InRegion {
                    region: (0, "check_num").into(),
                    offset: 0
                },
                cell_values: vec![(
                    ((Any::Advice(Advice::default()), 1).into(), 0).into(),
                    "0xa".to_string()
                )]
            }])
        );
    }
}
