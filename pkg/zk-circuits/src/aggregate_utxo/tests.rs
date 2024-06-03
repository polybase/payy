use crate::test::rollup::Rollup;
use halo2_base::halo2_proofs::{dev::MockProver, halo2curves::bn256::Fr};

#[test]
fn test_aggregate_utxo() {
    let k = 21;

    let mut rollup = Rollup::new();
    let bob = rollup.new_wallet();
    let alice = rollup.new_wallet();
    let sally = rollup.new_wallet();

    // Add existing notes to the tree
    let bob_n1 = rollup.unverified_add_unspent_note(&bob, 100);
    let bob_n2 = rollup.unverified_add_unspent_note(&bob, 100);
    let alice_n3 = rollup.unverified_add_unspent_note(&alice, 100);

    // Send 100 from bob to sally
    let utxo1 = rollup.transfer(bob_n1, sally.new_note(100));

    // Send 100 from bob to alice
    let utxo2 = rollup.transfer(bob_n2, alice.new_note(100));

    // Send 100 from alice to sally
    let utxo3 = rollup.transfer(alice_n3, sally.new_note(100));

    // Aggregate UTXOs
    let utxos = [utxo1, utxo2, utxo3];
    let aggregate_utxo = rollup.aggregate_utxo(&utxos);

    let prover =
        MockProver::<Fr>::run(k, &aggregate_utxo, vec![aggregate_utxo.public_inputs()]).unwrap();

    prover.assert_satisfied();
}
