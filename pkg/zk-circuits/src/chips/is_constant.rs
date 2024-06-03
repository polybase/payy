use super::is_zero::{IsZeroChip, IsZeroConfig};
use halo2_base::halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Column, ConstraintSystem, Constraints, Error, Expression, Selector},
    poly::Rotation,
};

#[derive(Clone, Debug)]
pub struct IsConstantConfig<F: FieldExt> {
    is_zero_config: IsZeroConfig<F>,
    selector: Selector,
    zero_advice: Column<Advice>,
    output_advice: Column<Advice>,
    constant: F,
}

#[derive(Clone, Debug)]
pub struct IsConstantChip<F: FieldExt> {
    config: IsConstantConfig<F>,
}

impl<F: FieldExt> IsConstantChip<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        zero_advice: Column<Advice>,
        inverse_advice: Column<Advice>,
        output_advice: Column<Advice>,
        constant: F,
    ) -> IsConstantConfig<F> {
        let selector: halo2_base::halo2_proofs::plonk::Selector = meta.selector();

        let is_zero_config = IsZeroChip::<F>::configure(
            meta,
            |meta| meta.query_selector(selector),
            |meta| meta.query_advice(zero_advice, Rotation::cur()) - Expression::Constant(constant),
            inverse_advice,
        );

        meta.create_gate("is_constant", |meta| {
            let s = meta.query_selector(selector);
            let o = meta.query_advice(output_advice, Rotation::cur());

            Constraints::with_selector(s, [o - is_zero_config.is_zero_expr.clone()])
        });

        IsConstantConfig {
            is_zero_config,
            selector,
            zero_advice,
            output_advice,
            constant,
        }
    }

    pub fn construct(config: IsConstantConfig<F>) -> Self {
        IsConstantChip { config }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        comparison: AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        let zero_chip = IsZeroChip::construct(self.config.is_zero_config.clone());

        layouter.assign_region(
            || "check is constant",
            |mut region| {
                // Enable the selector
                self.config.selector.enable(&mut region, 0)?;

                // Copy the assigned cell into the region
                let comparison_cell: AssignedCell<F, F> = comparison.copy_advice(
                    || "comparison value",
                    &mut region,
                    self.config.zero_advice,
                    0,
                )?;

                // Subtract the constant from the provided cell value
                let value = comparison_cell.value().cloned() - Value::known(self.config.constant);

                // Assign the inverse value
                zero_chip.assign(&mut region, 0, value)?;

                // Assign the bool value
                let output_cell = region.assign_advice(
                    || "is constant",
                    self.config.output_advice,
                    0,
                    || {
                        value.and_then(|v| {
                            if v == F::zero() {
                                Value::known(F::one())
                            } else {
                                Value::known(F::zero())
                            }
                        })
                    },
                )?;

                Ok(output_cell)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use halo2_base::halo2_proofs::{
        circuit::SimpleFloorPlanner,
        dev::MockProver,
        halo2curves::bn256::Fr,
        plonk::{Circuit, Instance},
    };

    use crate::{
        test::util::{advice_column_equality, instance_column_equality},
        util::assign_private_input,
    };

    use super::*;

    #[derive(Clone, Debug)]
    struct IsConstantCircuitConfig {
        is_constant_config: IsConstantConfig<Fr>,
        instance: Column<Instance>,
        comparison: Column<Advice>,
    }

    #[derive(Default, Clone, Debug)]
    struct IsConstantCircuit {
        compare: Fr,
    }

    impl Circuit<Fr> for IsConstantCircuit {
        type Config = IsConstantCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            let zero_advice = advice_column_equality(meta);
            let inverse_advice = advice_column_equality(meta);
            let output_advice = advice_column_equality(meta);

            IsConstantCircuitConfig {
                is_constant_config: IsConstantChip::configure(
                    meta,
                    zero_advice,
                    inverse_advice,
                    output_advice,
                    Fr::from_u128(10u128),
                ),
                comparison: advice_column_equality(meta),
                instance: instance_column_equality(meta),
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let is_constant_chip = IsConstantChip::construct(config.is_constant_config);

            let comparison_witness = assign_private_input(
                || "witness compare",
                layouter.namespace(|| "witness compare"),
                config.comparison,
                Value::known(self.compare),
            )?;

            let output = is_constant_chip.assign(
                layouter.namespace(|| "compare to constant"),
                comparison_witness,
            )?;

            layouter.constrain_instance(output.cell(), config.instance, 0)?;

            Ok(())
        }
    }

    #[test]
    fn test_equal_constant() {
        let k = 3;

        let public_input = vec![Fr::from_u128(1u128)];
        let instance_columns = vec![public_input];
        let circuit = IsConstantCircuit {
            compare: Fr::from_u128(10u128),
        };

        let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_not_equal_constant() {
        let k = 3;

        let public_input = vec![Fr::from_u128(0u128)];
        let instance_columns = vec![public_input];
        let circuit = IsConstantCircuit {
            compare: Fr::from_u128(11u128),
        };

        let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_zero() {
        let k = 3;

        let public_input = vec![Fr::from_u128(0u128)];
        let instance_columns = vec![public_input];
        let circuit = IsConstantCircuit {
            compare: Fr::from_u128(0u128),
        };

        let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();
    }
}
