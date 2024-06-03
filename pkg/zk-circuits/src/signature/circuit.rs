use crate::{chips::poseidon::{P128Pow5T3Fr, PoseidonChip, PoseidonConfig}, data::Signature};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    halo2curves::bn256::Fr,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};

#[derive(Clone, Debug)]
pub struct SignatureCircuitConfig {
    advices: [Column<Advice>; 5],
    instance: Column<Instance>,
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
}

impl Circuit<Fr> for Signature {
    type FloorPlanner = SimpleFloorPlanner;
    type Config = SignatureCircuitConfig;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        let instance = meta.instance_column();
        meta.enable_equality(instance);

        let advices = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];

        for advice in advices.iter() {
            meta.enable_equality(*advice);
        }

        let lagrange_coeffs = [
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
        ];
        meta.enable_constant(lagrange_coeffs[0]);

        let poseidon_config = PoseidonChip::configure::<P128Pow5T3Fr>(
            meta,
            advices[1..4].try_into().unwrap(),
            advices[0],
            lagrange_coeffs[0..3].try_into().unwrap(),
            lagrange_coeffs[3..6].try_into().unwrap(),
        );

        SignatureCircuitConfig {
            advices,
            instance,
            poseidon_config,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        self.enforce_constraints(
            layouter.namespace(|| "signature"),
            config.advices[0],
            config.instance,
            config.poseidon_config,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use halo2_base::halo2_proofs::dev::MockProver;
    use rand::thread_rng;
    use smirk::Element;

    use crate::{test::util::get_params, Snark};

    use super::*;

    #[test]
    fn test_address_proof() {
        let k = 6;

        let pk = Element::secure_random(thread_rng());
        let message = Element::secure_random(thread_rng());

        let circuit = Signature::new(pk, message);
        let instance_columns = vec![circuit.public_inputs()];

        // Prove mock
        let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
        prover.assert_satisfied();

        // Prove for real circuit
        let (params, _vk, pk) = get_params(k, &circuit);
        let _snark =
            Snark::create(circuit.clone(), vec![circuit.public_inputs()], &params, &pk).unwrap();
    }
}
