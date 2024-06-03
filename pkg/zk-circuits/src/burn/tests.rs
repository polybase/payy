use crate::data::{Burn, ParameterSet};
use crate::evm_verifier;
use crate::test::{rollup::Rollup, util::get_params};
use crate::util::keygen_from_params;
use halo2_base::halo2_proofs::dev::MockProver;
use halo2_base::halo2_proofs::halo2curves::bn256::Fr;
use smirk::Element;

#[test]
fn test_burn_proof() {
    let k = 9;

    let mut rollup = Rollup::new();
    let bob = rollup.new_wallet();

    let bob_note = rollup.unverified_add_unspent_note(&bob, 100);

    let circuit = {
        let notes = [bob_note.note()];
        let secret_key = Element::ONE;
        let to_address = Element::ONE;
        Burn {
            notes,
            secret_key,
            to_address,
        }
    };
    let instance_columns = vec![circuit.public_inputs()];

    // Prove mockw
    let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
    prover.assert_satisfied();

    // Prove for real circuit
    let (params, _, pk) = get_params(k, &circuit);
    circuit
        .prove(&params, &pk, &mut rand::thread_rng())
        .unwrap();
}

#[test]
fn generate_verifier() {
    let params_9 = ParameterSet::Nine;

    let circuit = Burn::<1>::default();

    let (pk, _) = keygen_from_params(params_9, &circuit);
    let yul_code =
        evm_verifier::generate_verifier(params_9, &pk, vec![circuit.public_inputs().len()]);

    let expected_yul_code = expect_test::expect_file!["./burn_verifier.yul"];
    expected_yul_code.assert_eq(&yul_code);
}
