use crate::data::{Mint, Note, ParameterSet};
use crate::evm_verifier;
use crate::test::util::get_params;
use crate::util::{insecure_random_element, keygen_from_params};
use halo2_base::halo2_proofs::dev::MockProver;
use halo2_base::halo2_proofs::halo2curves::bn256::Fr;
use zk_primitives::Element;

#[test]
fn test_mint_proof() {
    let k = 8;

    let address = insecure_random_element();
    let note = Note::new(address, Element::from(100u64));

    let circuit = Mint::new([note]);
    let instance_columns = vec![circuit.public_inputs()];

    // Prove mock
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
    let params_8 = ParameterSet::Eight;

    let circuit = Mint::<1>::default();

    let (pk, _) = keygen_from_params(params_8, &circuit);
    let yul_code =
        evm_verifier::generate_verifier(params_8, &pk, vec![circuit.public_inputs().len()]);

    let expected_yul_code = expect_test::expect_file!["./mint_verifier.yul"];
    expected_yul_code.assert_eq(&yul_code);
}
