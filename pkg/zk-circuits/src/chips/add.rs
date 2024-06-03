use halo2_base::halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct AddCulmChipConfig {
    selector: Selector,
    culm: Column<Advice>,
    add: Column<Advice>,
}

#[derive(Debug, Clone)]
pub struct AddCulmChip<F: Field> {
    config: AddCulmChipConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> AddCulmChip<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        add: Column<Advice>,
        culm: Column<Advice>,
    ) -> AddCulmChipConfig {
        let selector = meta.selector();

        meta.create_gate("Sum", |meta| {
            let s = meta.query_selector(selector);
            let a = meta.query_advice(culm, Rotation::prev());
            let b = meta.query_advice(add, Rotation::cur());
            let c = meta.query_advice(culm, Rotation::cur());

            //  add   |  culm   | selector
            //        |  a      |
            //   b    |  c      | 1

            vec![s * (a + b - c)]
        });

        AddCulmChipConfig {
            selector,
            culm,
            add,
        }
    }

    pub fn construct(config: AddCulmChipConfig) -> Self {
        Self {
            _marker: PhantomData,
            config,
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        cells: &[AssignedCell<F, F>],
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "add chip",
            |mut region| {
                let first = &cells[0];

                let mut culm =
                    first.copy_advice(|| "first culm", &mut region, self.config.culm, 0)?;

                for (i, cell) in cells[1..].iter().enumerate() {
                    let offset = i + 1;

                    // Enable the selector, so we constrain the new culm value
                    self.config.selector.enable(&mut region, offset)?;

                    // Copy in the value to be added
                    let b = cell.copy_advice(|| "add", &mut region, self.config.add, offset)?;

                    // Add the expected value
                    culm = region.assign_advice(
                        || "new culm",
                        self.config.culm,
                        offset,
                        || {
                            culm.value().and_then(|culm_val| {
                                b.value().and_then(|b_val| Value::known(*culm_val + *b_val))
                            })
                        },
                    )?
                }

                // Return the final calculated value
                Ok(culm)
            },
        )
    }
}
