use crate::{
    chips::aggregation::{
        aggregate::{accumulator_native, AggregationChip},
        snark::Snark,
    },
    data::{AggregateAgg, ParameterSet},
    params::load_params,
    util::keygen_from_params,
    CircuitKind,
};
use halo2_base::halo2_proofs::{
    circuit::{Cell, Layouter, Value},
    halo2curves::bn256::{Fr, G1Affine},
    plonk::{Column, Error, Instance, ProvingKey, VerifyingKey},
};
use itertools::Itertools;
use smirk::Element;

impl<const AGG_N: usize> AggregateAgg<AGG_N> {
    pub fn new(aggregates: [Snark; AGG_N]) -> Self {
        let snarks: Vec<&Snark> = Self::snarks(&aggregates);

        let (agg_instances, proof) = accumulator_native(&snarks);
        let agg_instances = agg_instances.into_iter().map(Element::from).collect();

        Self {
            aggregates,
            agg_instances,
            proof,
        }
    }

    fn snarks(utxo: &[Snark; AGG_N]) -> Vec<&Snark> {
        utxo.iter().collect_vec()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        instance: Column<Instance>,
        aggregation_chip: &AggregationChip,
    ) -> Result<(), Error> {
        // Gather snarks
        let snarks = Self::snarks(&self.aggregates);

        // Aggregate proofs
        let (agg_cells, aggregates) = aggregation_chip.aggregate(
            layouter.namespace(|| "aggregate"),
            &snarks,
            Value::known(&self.proof),
        )?;

        // Constrain verify aggregation cells to public inputs
        for (i, cell) in agg_cells.iter().enumerate() {
            layouter.constrain_instance(*cell, instance, i)?;
        }

        let old_root = aggregates[0][0][12];
        let mut last_new_root = old_root;
        let mut recent_roots: Vec<Cell> = vec![];

        // Prove new root is next old root
        for agg in aggregates {
            let old_root = agg[0][12];
            let new_root = agg[0][13];
            recent_roots.extend(agg[0][14..].iter());

            // Check that the old root is the same as the last new root
            layouter.assign_region(
                || "constrain roots",
                |mut region| region.constrain_equal(last_new_root, old_root),
            )?;

            last_new_root = new_root;
        }

        // Constrain old root
        layouter.constrain_instance(old_root, instance, 12)?;

        // Constrain new root
        layouter.constrain_instance(last_new_root, instance, 13)?;

        // Constraint recent roots (pass through)
        for (i, recent_root) in recent_roots.iter().enumerate() {
            layouter.constrain_instance(*recent_root, instance, 14 + i)?;
        }

        Ok(())
    }

    // TODO: we should use a typed system for extracting cells from the snark instances
    pub fn public_inputs(&self) -> Vec<Fr> {
        let mut instances = vec![];

        // Add verify instances (12)
        instances.extend(self.agg_instances.iter().copied().map(Fr::from));

        // Add old root (1)
        instances.push(*self.old_root());

        // Add new root (1)
        instances.push(*self.new_root());

        // UTXO values (recent root, mint/burn hash, mint/burn value) (= 3 per UTXO)
        instances.extend(self.utxo_values());

        instances
    }

    pub fn agg_instances(&self) -> &Vec<Element> {
        &self.agg_instances
    }

    pub fn old_root(&self) -> &Fr {
        &self.aggregates[0].instances[0][12]
    }

    pub fn new_root(&self) -> &Fr {
        &self.aggregates.iter().last().unwrap().instances[0][13]
    }

    pub fn utxo_values(&self) -> Vec<Fr> {
        self.aggregates
            .iter()
            .flat_map(|snark| &snark.instances[0][14..])
            .copied()
            .collect_vec()
    }

    pub fn snark(&self, params: ParameterSet) -> Result<Snark, crate::Error> {
        let pk = CircuitKind::AggAgg.pk();
        Snark::create(
            self.clone(),
            vec![self.public_inputs()],
            load_params(params),
            pk,
        )
        .map_err(crate::Error::err)
    }

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }
}
