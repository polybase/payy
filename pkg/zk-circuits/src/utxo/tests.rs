use crate::{
    constants::MERKLE_TREE_DEPTH,
    data::{InputNote, Note, Utxo, UtxoKind},
    test::rollup::Rollup,
    CircuitKind,
};
use halo2_base::halo2_proofs::{dev::MockProver, halo2curves::bn256::Fr};

#[test]
fn test_utxo_one_input_one_output() {
    let k = 14;

    let mut rollup = Rollup::new();
    let bob = rollup.new_wallet();
    let alice = rollup.new_wallet();

    // Add existing note to the tree for bob
    let bob_note = rollup.unverified_add_unspent_note(&bob, 10);
    let recent_root = rollup.root_hash();

    let input_note = rollup.to_input_note(&bob_note);
    let input_notes = [input_note.clone(), InputNote::padding_note()];

    let output_note = alice.new_note(10);
    let output_notes = [output_note.clone(), Note::padding_note()];

    let circuit = Utxo::new(input_notes, output_notes, recent_root, UtxoKind::Transfer);
    let public_input = circuit.public_inputs();

    assert_eq!(public_input.len(), 7);
    assert_eq!(public_input[0], recent_root.to_base());
    assert_eq!(public_input[1], Fr::zero());
    assert_eq!(public_input[2], Fr::zero());
    assert_eq!(public_input[3], input_note.nullifer().into());
    assert_eq!(public_input[4], Fr::zero());
    assert_eq!(public_input[5], output_note.commitment().into());
    assert_eq!(public_input[6], Fr::zero());

    let instance_columns = vec![public_input];

    // Prove mock
    let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
    prover.assert_satisfied();

    // Prove for real circuit
    let snark = circuit.snark(CircuitKind::Utxo).unwrap();
    assert!(snark.to_witness().verify(CircuitKind::Utxo));
}

#[test]
fn test_utxo_one_input_two_output() {
    let k = 14;

    let mut rollup = Rollup::new();
    let bob = rollup.new_wallet();
    let alice = rollup.new_wallet();
    let sally = rollup.new_wallet();

    // Add existing note to the tree for bob
    let bob_note = rollup.unverified_add_unspent_note(&bob, 100);
    let recent_root = rollup.root_hash();

    let input_note = rollup.to_input_note(&bob_note);
    let input_notes = [input_note.clone(), InputNote::padding_note()];

    let output_notes = [alice.new_note(30), sally.new_note(70)];

    let circuit = Utxo::new(
        input_notes,
        output_notes.clone(),
        recent_root,
        UtxoKind::Transfer,
    );
    let public_input = circuit.public_inputs();

    assert_eq!(public_input.len(), 7);
    assert_eq!(public_input[0], recent_root.to_base());
    assert_eq!(public_input[1], Fr::zero());
    assert_eq!(public_input[2], Fr::zero());
    assert_eq!(public_input[3], input_note.nullifer().into());
    assert_eq!(public_input[4], Fr::zero());
    assert_eq!(public_input[5], output_notes[0].commitment().into());
    assert_eq!(public_input[6], output_notes[1].commitment().into());

    let instance_columns = vec![public_input];

    let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
    prover.assert_satisfied();
}

#[test]
fn test_utxo_mint() {
    let k = 14;

    let rollup = Rollup::new();
    let bob = rollup.new_wallet();

    let output_note = bob.new_note(100);

    let circuit = Utxo::<MERKLE_TREE_DEPTH>::new_mint(output_note.clone());
    let public_input = circuit.public_inputs();

    assert_eq!(public_input.len(), 7);
    assert_eq!(public_input[0], Fr::zero());
    assert_eq!(public_input[1], output_note.commitment().into());
    assert_eq!(public_input[2], Fr::from(100u64));
    assert_eq!(public_input[3], Fr::zero());
    assert_eq!(public_input[4], Fr::zero());
    assert_eq!(public_input[5], output_note.commitment().into());
    assert_eq!(public_input[6], Fr::zero());

    let instance_columns = vec![public_input];

    let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
    prover.assert_satisfied();
}

#[test]
fn test_utxo_burn() {
    let k = 14;

    let mut rollup = Rollup::new();
    let bob = rollup.new_wallet();

    // Add existing note to the tree for bob
    let bob_note = rollup.unverified_add_unspent_note(&bob, 100);
    let recent_root = rollup.root_hash();

    let input_note = rollup.to_input_note(&bob_note);

    let circuit = Utxo::new_burn(input_note.clone(), recent_root);
    let public_input = circuit.public_inputs();

    assert_eq!(public_input.len(), 7);
    assert_eq!(public_input[0], recent_root.to_base());
    assert_eq!(public_input[3], input_note.nullifer().into());
    assert_eq!(public_input[2], Fr::from(100u64));
    assert_eq!(public_input[3], input_note.nullifer().into());
    assert_eq!(public_input[4], Fr::zero());
    assert_eq!(public_input[5], Fr::zero());
    assert_eq!(public_input[6], Fr::zero());

    let instance_columns = vec![public_input];

    let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
    prover.assert_satisfied();
}
