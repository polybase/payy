use crate::data::{Note, Points};
use halo2_base::halo2_proofs::{dev::MockProver, halo2curves::bn256::Fr};
use rand::thread_rng;
use smirk::{hash_merge, Element};
use snark_verifier::util::arithmetic::FieldExt;

#[test]
fn test_one_note() {
    let k = 14;

    let pk = Element::secure_random(thread_rng());
    let address = hash_merge([pk, Element::ZERO]);

    let mut notes = vec![
        Note::new(address, Element::from(10u32)),
        Note::new(address, Element::from(5u32)),
        Note::new(address, Element::from(6u32)),
        Note::new(address, Element::from(10u32)),
    ];

    for _ in 0..112 - 4 {
        notes.push(Note::padding_note())
    }

    let circuit = Points::new(pk, notes);
    let public_input = circuit.public_inputs();

    assert_eq!(public_input.len(), 2 + (112 * 2));
    assert_eq!(public_input[0], address.to_base());
    assert_eq!(public_input[1], Fr::from_u128(31));

    let instance_columns = vec![public_input];

    // Prove mock
    let prover = MockProver::<Fr>::run(k, &circuit, instance_columns).unwrap();
    prover.assert_satisfied();
}
