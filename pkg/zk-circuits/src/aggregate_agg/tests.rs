use crate::{
    data::{AggregateAgg, ParameterSet},
    evm_verifier,
    test::{agg_agg::create_or_load_agg_agg_utxo_snark, agg_utxo::create_or_load_agg_utxo_snarks},
};
use halo2_base::halo2_proofs::{dev::MockProver, halo2curves::bn256::Fr};

#[test]
fn test_aggregate_agg() {
    let k = 21;

    let utxo_aggs = create_or_load_agg_utxo_snarks(ParameterSet::TwentyOne);

    let aggregate_agg = AggregateAgg::<2>::new(utxo_aggs);

    let prover =
        MockProver::<Fr>::run(k, &aggregate_agg, vec![aggregate_agg.public_inputs()]).unwrap();

    prover.assert_satisfied();
}

#[test]
fn generate_verifier() {
    let params_21 = ParameterSet::TwentyOne;

    let utxo_aggs = create_or_load_agg_utxo_snarks(params_21);
    let aggregate_agg = create_or_load_agg_agg_utxo_snark(params_21, utxo_aggs);

    // Currently we can only do 1 for the Ethereum verifier as 2 creates a "too large" verifier (25,137 bytes) where
    // the max limit is 24,576 bytes (we are so close, we might be able to get this to fit!)
    let aggregate_agg_agg = AggregateAgg::<1>::new([aggregate_agg]);

    let pk = aggregate_agg_agg.keygen(params_21).0;

    let yul_code = evm_verifier::generate_verifier(
        params_21,
        &pk,
        vec![aggregate_agg_agg.public_inputs().len()],
    );

    let expected_yul_code = expect_test::expect_file!["./aggregate_verifier.yul"];
    expected_yul_code.assert_eq(&yul_code);
}
