use element::Element;
use flate2::{write::GzEncoder, Compression};
use std::io::Write;
use zk_primitives::{
    get_address_for_private_key, AggAgg, AggUtxo, InputNote, MerklePath, Note, ToBytes, Utxo,
    UtxoKind, UtxoProof, UtxoProofBundleWithMerkleProofs,
};

use crate::{circuits::agg_test::AggTestInput, Prove, Result, Verify};

use super::AGG_UTXO_VERIFICATION_KEY_HASH;

pub fn get_keypair(key: u64) -> (Element, Element) {
    let secret_key = Element::new(key);
    let address = get_address_for_private_key(secret_key);
    (secret_key, address)
}

pub fn send_note(value: u64, address: Element, psi: u64) -> Note {
    note(value, address, psi, 1)
}

pub fn note(value: u64, address: Element, psi: u64, kind: u64) -> Note {
    Note {
        value: Element::new(value),
        address,
        kind: Element::new(kind),
        psi: Element::new(psi),
    }
}

pub fn compress_proof(proof: &impl ToBytes) -> Vec<u8> {
    let mut proof_gz = Vec::new();
    let mut encoder = GzEncoder::new(&mut proof_gz, Compression::best());
    encoder.write_all(proof.to_bytes().as_slice()).unwrap();
    encoder.finish().unwrap();
    proof_gz
}

pub fn prove_proof<P: Prove>(proof_input: &P) -> Result<P::Proof> {
    let start = std::time::Instant::now();
    let proof = proof_input.prove().unwrap();
    let end = std::time::Instant::now() - start;
    println!("Proving completed in {:?}", end);
    let proof_gz = compress_proof(&proof);
    println!(
        "Proof size: {:?} (compressed: {:?})",
        proof.to_bytes().len(),
        &proof_gz.len()
    );
    Ok(proof)
}

pub fn verify_proof(proof: &impl Verify) {
    let start = std::time::Instant::now();
    let result = proof.verify();
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Proof verification failed: {:?}",
        result.err()
    );

    println!("Proof verification completed in {:?}", duration);
}

pub fn prove_and_verify<P: Prove>(proof_input: &P) -> Result<P::Proof> {
    let proof = prove_proof(proof_input)?;
    verify_proof(&proof);
    Ok(proof)
}

#[test]
fn test_utxo() {
    let (secret_key, address) = get_keypair(101);

    let input_note1 = InputNote {
        note: send_note(50, address, 1),
        secret_key,
    };

    let input_note2 = InputNote {
        note: send_note(30, address, 2),
        secret_key,
    };

    let output_note1 = send_note(40, address, 3);
    let output_note2 = send_note(40, address, 4);

    let utxo = Utxo {
        input_notes: [input_note1, input_note2],
        output_notes: [output_note1, output_note2],
        kind: UtxoKind::Send,
        burn_address: None,
    };

    prove_and_verify(&utxo).unwrap();
}

#[test]
fn test_agg_test() {
    let (secret_key, address) = get_keypair(101);

    let input_note1 = InputNote {
        note: send_note(50, address, 1),
        secret_key,
    };

    let input_note2 = InputNote {
        note: send_note(30, address, 2),
        secret_key,
    };

    let output_note1 = send_note(40, address, 3);
    let output_note2 = send_note(40, address, 4);

    let utxo = Utxo {
        input_notes: [input_note1, input_note2],
        output_notes: [output_note1, output_note2],
        kind: UtxoKind::Send,
        burn_address: None,
    };

    let utxo_proof = utxo.prove().unwrap();

    let agg_input = AggTestInput {
        proof: utxo_proof
            .proof
            .to_fields()
            .iter()
            .map(|e| e.to_base())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        public_inputs: utxo_proof
            .public_inputs
            .fields()
            .iter()
            .map(|e| e.to_base())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    };

    prove_and_verify(&agg_input).unwrap();
}

fn process_utxo_for_agg(
    tree: &mut smirk::Tree<161, ()>,
    utxo: &Utxo,
) -> Result<(
    UtxoProof,
    MerklePath<161>,
    MerklePath<161>,
    MerklePath<161>,
    MerklePath<161>,
    Element,
)> {
    let utxo_proof = utxo.prove()?;

    let p1: MerklePath<161> = MerklePath::new(
        tree.path_for(utxo.input_notes[0].note.commitment())
            .siblings
            .to_vec(),
    );
    tree.remove(utxo.input_notes[0].note.commitment()).unwrap();

    let p2: MerklePath<161> = MerklePath::new(
        tree.path_for(utxo.input_notes[1].note.commitment())
            .siblings
            .to_vec(),
    );
    tree.remove(utxo.input_notes[1].note.commitment()).unwrap();

    tree.insert(utxo.output_notes[0].commitment(), ()).unwrap();
    let p3: MerklePath<161> = MerklePath::new(
        tree.path_for(utxo.output_notes[0].commitment())
            .siblings
            .to_vec(),
    );

    tree.insert(utxo.output_notes[1].commitment(), ()).unwrap();
    let p4: MerklePath<161> = MerklePath::new(
        tree.path_for(utxo.output_notes[1].commitment())
            .siblings
            .to_vec(),
    );

    let new_root = tree.root_hash();

    Ok((utxo_proof, p1, p2, p3, p4, new_root))
}

#[test]
fn test_agg_utxo() {
    let (secret_key, address) = get_keypair(101);
    let mut tree = smirk::Tree::<161, ()>::new();

    let utxo1_input_note1 = InputNote {
        note: send_note(50, address, 1),
        secret_key,
    };
    tree.insert(utxo1_input_note1.note.commitment(), ())
        .unwrap();

    let utxo1_input_note2 = InputNote {
        note: send_note(30, address, 2),
        secret_key,
    };
    tree.insert(utxo1_input_note2.note.commitment(), ())
        .unwrap();

    let utxo1_old_root = tree.root_hash();

    let utxo1_output_note1 = send_note(40, address, 3);
    let utxo1_output_note2 = send_note(40, address, 4);

    let utxo1 = Utxo {
        input_notes: [utxo1_input_note1.clone(), utxo1_input_note2.clone()],
        output_notes: [utxo1_output_note1.clone(), utxo1_output_note2.clone()],
        kind: UtxoKind::Send,
        burn_address: None,
    };

    let (utxo1_proof, p1, p2, p3, p4, utxo1_new_root) =
        process_utxo_for_agg(&mut tree, &utxo1).unwrap();
    verify_proof(&utxo1_proof);

    let agg_utxo1 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo1_proof, &[p1, p2, p3, p4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo1_old_root,
        utxo1_new_root,
    );

    prove_and_verify(&agg_utxo1).unwrap();
}

#[test]
fn test_agg_agg() {
    let (secret_key, address) = get_keypair(101);
    let mut tree = smirk::Tree::<161, ()>::new();

    let utxo1_input_note1 = InputNote {
        note: send_note(60, address, 1),
        secret_key,
    };
    tree.insert(utxo1_input_note1.note.commitment(), ())
        .unwrap();

    let utxo1_input_note2 = InputNote {
        note: send_note(40, address, 2),
        secret_key,
    };
    tree.insert(utxo1_input_note2.note.commitment(), ())
        .unwrap();

    let utxo1_old_root = tree.root_hash();

    let utxo1_output_note1 = send_note(70, address, 3);
    let utxo1_output_note2 = send_note(30, address, 4);

    let utxo1 = Utxo {
        input_notes: [utxo1_input_note1.clone(), utxo1_input_note2.clone()],
        output_notes: [utxo1_output_note1.clone(), utxo1_output_note2.clone()],
        kind: UtxoKind::Send,
        burn_address: None,
    };

    let (utxo1_proof, p1_1, p1_2, p1_3, p1_4, utxo1_new_root) =
        process_utxo_for_agg(&mut tree, &utxo1).unwrap();
    verify_proof(&utxo1_proof);

    let agg_utxo1 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo1_proof, &[p1_1, p1_2, p1_3, p1_4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo1_old_root,
        utxo1_new_root,
    );
    let agg_utxo1_proof = prove_and_verify(&agg_utxo1).unwrap();

    let utxo2_input_note1 = InputNote {
        note: utxo1_output_note1.clone(),
        secret_key,
    };

    let utxo2_input_note2 = InputNote {
        note: utxo1_output_note2.clone(),
        secret_key,
    };

    let utxo2_old_root = tree.root_hash();

    let utxo2_output_note1 = send_note(55, address, 5);
    let utxo2_output_note2 = send_note(45, address, 6);

    let utxo2 = Utxo {
        input_notes: [utxo2_input_note1.clone(), utxo2_input_note2.clone()],
        output_notes: [utxo2_output_note1.clone(), utxo2_output_note2.clone()],
        kind: UtxoKind::Send,
        burn_address: None,
    };

    let (utxo2_proof, p2_1, p2_2, p2_3, p2_4, utxo2_new_root) =
        process_utxo_for_agg(&mut tree, &utxo2).unwrap();
    verify_proof(&utxo2_proof);

    let agg_utxo2 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo2_proof, &[p2_1, p2_2, p2_3, p2_4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo2_old_root,
        utxo2_new_root,
    );
    let agg_utxo2_proof = prove_and_verify(&agg_utxo2).unwrap();

    let agg_agg = AggAgg::new(
        [agg_utxo1_proof, agg_utxo2_proof],
        Element::from_base(AGG_UTXO_VERIFICATION_KEY_HASH.0),
    );

    prove_and_verify(&agg_agg).unwrap();
}
