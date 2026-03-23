// lint-long-file-override allow-max-lines=700
use barretenberg_api_client::ClientBackend;
use barretenberg_cli::CliBackend;
use barretenberg_rs::BindingBackend;
use element::Element;
use std::sync::OnceLock;
use tokio::sync::{Mutex, MutexGuard};
use zk_primitives::{
    AggAgg, AggFinal, AggProof, AggUtxo, InputNote, MerklePath, Note, Utxo, UtxoKind, UtxoProof,
    UtxoProofBundleWithMerkleProofs, bridged_polygon_usdc_note_kind, get_address_for_private_key,
};

use crate::{BbBackend, Prove, Result, Verify, circuits::Proof};
use url::Url;

mod erc_20;
mod migrate;
mod points;
mod signature;

const API_BACKEND_ENV: &str = "BARRETENBERG_API_BASE_URL";
const RS_BACKEND_ENV: &str = "BARRETENBERG_RS";

/// Used to ensure that only one BbBackend exists at once
/// in order to limit memory usage of the tests.
static TEST_BACKEND_LOCK: OnceLock<Mutex<Box<dyn BbBackend>>> = OnceLock::new();

async fn test_backend() -> MutexGuard<'static, Box<dyn BbBackend>> {
    TEST_BACKEND_LOCK
        .get_or_init(|| Mutex::new(test_backend_inner()))
        .lock()
        .await
}

fn test_backend_inner() -> Box<dyn BbBackend> {
    if std::env::var(RS_BACKEND_ENV).is_ok() {
        return Box::new(BindingBackend);
    }

    match std::env::var(API_BACKEND_ENV) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Box::new(CliBackend)
            } else {
                match Url::parse(trimmed) {
                    Ok(url) => match ClientBackend::new(url) {
                        Ok(client) => Box::new(client),
                        Err(err) => {
                            eprintln!(
                                "Failed to initialize API backend ({err}), falling back to CLI backend"
                            );
                            Box::new(CliBackend)
                        }
                    },
                    Err(err) => {
                        eprintln!(
                            "Invalid {API_BACKEND_ENV} value `{trimmed}` ({err}), falling back to CLI backend"
                        );
                        Box::new(CliBackend)
                    }
                }
            }
        }
        Err(_) => Box::new(CliBackend),
    }
}

pub fn get_keypair(key: u64) -> (Element, Element) {
    let secret_key = Element::new(key);
    let address = get_address_for_private_key(secret_key);
    (secret_key, address)
}

pub fn send_note(value: u64, address: Element, psi: u64) -> Note {
    note(value, address, psi, bridged_polygon_usdc_note_kind())
}

pub fn note(value: u64, address: Element, psi: u64, contract: Element) -> Note {
    Note {
        kind: Element::new(2),
        value: Element::new(value),
        address,
        contract,
        psi: Element::new(psi),
    }
}

pub async fn prove_proof<P: Prove>(proof_input: &P) -> Result<P::Proof> {
    let backend = test_backend().await;
    let start = std::time::Instant::now();
    let proof = proof_input.prove(backend.as_ref()).await.unwrap();
    let end = std::time::Instant::now() - start;
    println!("Proving completed in {end:?}");
    Ok(proof)
}

pub async fn verify_proof(proof: &impl Verify) {
    let backend = test_backend().await;
    let start = std::time::Instant::now();
    let result = proof.verify(backend.as_ref()).await;
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Proof verification failed: {:?}",
        result.err()
    );

    println!("Proof verification completed in {duration:?}");
}

pub async fn verify_utxo_proof(proof: &UtxoProof) {
    let circuit_proof = Proof::from(proof.clone());
    verify_proof(&circuit_proof).await;
}

pub async fn prove_and_verify<P: Prove>(proof_input: &P) -> Result<P::Proof> {
    let proof = prove_proof(proof_input).await?;
    verify_proof(&proof).await;
    Ok(proof)
}

async fn process_utxo_for_agg(
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
    let backend = test_backend().await;
    let utxo_proof = utxo.prove(backend.as_ref()).await?;

    let input_commitment_1 = utxo.input_notes[0].note.commitment();
    let p1: MerklePath<161> = MerklePath::new(tree.path_for(input_commitment_1).siblings.to_vec());
    if input_commitment_1 != Element::ZERO {
        tree.remove(input_commitment_1).unwrap();
    }

    let input_commitment_2 = utxo.input_notes[1].note.commitment();
    let p2: MerklePath<161> = MerklePath::new(tree.path_for(input_commitment_2).siblings.to_vec());
    if input_commitment_2 != Element::ZERO {
        tree.remove(input_commitment_2).unwrap();
    }

    let output_commitment_1 = utxo.output_notes[0].commitment();
    if output_commitment_1 != Element::ZERO {
        tree.insert(output_commitment_1, ()).unwrap();
    }
    let p3: MerklePath<161> = MerklePath::new(tree.path_for(output_commitment_1).siblings.to_vec());

    let output_commitment_2 = utxo.output_notes[1].commitment();
    if output_commitment_2 != Element::ZERO {
        tree.insert(output_commitment_2, ()).unwrap();
    }
    let p4: MerklePath<161> = MerklePath::new(tree.path_for(output_commitment_2).siblings.to_vec());

    let new_root = tree.root_hash();

    Ok((utxo_proof, p1, p2, p3, p4, new_root))
}

pub fn add_note_to_tree(tree: &mut smirk::Tree<161, ()>, note: &InputNote) {
    tree.insert(note.note.commitment(), ()).unwrap();
}

#[tokio::test]
async fn test_utxo() {
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

    prove_and_verify(&utxo).await.unwrap();
}

#[tokio::test]
async fn test_agg_utxo() {
    let (secret_key, address) = get_keypair(101);
    let mut tree = smirk::Tree::<161, ()>::new();

    let utxo1_input_note1 = InputNote {
        note: send_note(50, address, 1),
        secret_key,
    };
    add_note_to_tree(&mut tree, &utxo1_input_note1);

    let utxo1_input_note2 = InputNote {
        note: send_note(30, address, 2),
        secret_key,
    };
    add_note_to_tree(&mut tree, &utxo1_input_note2);

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
        process_utxo_for_agg(&mut tree, &utxo1).await.unwrap();
    verify_utxo_proof(&utxo1_proof).await;

    let agg_utxo1 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo1_proof, &[p1, p2, p3, p4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo1_old_root,
        utxo1_new_root,
    );

    prove_and_verify(&agg_utxo1).await.unwrap();
}

#[tokio::test]
async fn test_agg_agg() {
    let (secret_key, address) = get_keypair(101);
    let mut tree = smirk::Tree::<161, ()>::new();

    let utxo1_input_note1 = InputNote {
        note: send_note(60, address, 1),
        secret_key,
    };
    add_note_to_tree(&mut tree, &utxo1_input_note1);

    let utxo1_input_note2 = InputNote {
        note: send_note(40, address, 2),
        secret_key,
    };
    add_note_to_tree(&mut tree, &utxo1_input_note2);

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
        process_utxo_for_agg(&mut tree, &utxo1).await.unwrap();
    verify_utxo_proof(&utxo1_proof).await;

    let agg_utxo1 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo1_proof, &[p1_1, p1_2, p1_3, p1_4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo1_old_root,
        utxo1_new_root,
    );
    let agg_utxo1_proof = prove_and_verify(&agg_utxo1).await.unwrap();

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
        process_utxo_for_agg(&mut tree, &utxo2).await.unwrap();
    verify_utxo_proof(&utxo2_proof).await;

    let agg_utxo2 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo2_proof, &[p2_1, p2_2, p2_3, p2_4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo2_old_root,
        utxo2_new_root,
    );
    let agg_utxo2_proof = prove_and_verify(&agg_utxo2).await.unwrap();

    let agg_agg = AggAgg::new([
        AggProof::AggUtxo(Box::new(agg_utxo1_proof)),
        AggProof::AggUtxo(Box::new(agg_utxo2_proof)),
    ]);

    prove_and_verify(&agg_agg).await.unwrap();
}

#[tokio::test]
async fn test_agg_final() {
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
        process_utxo_for_agg(&mut tree, &utxo1).await.unwrap();
    verify_utxo_proof(&utxo1_proof).await;

    let agg_utxo1 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo1_proof, &[p1_1, p1_2, p1_3, p1_4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo1_old_root,
        utxo1_new_root,
    );
    let agg_utxo1_proof = prove_and_verify(&agg_utxo1).await.unwrap();

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
        process_utxo_for_agg(&mut tree, &utxo2).await.unwrap();
    verify_utxo_proof(&utxo2_proof).await;

    let agg_utxo2 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo2_proof, &[p2_1, p2_2, p2_3, p2_4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo2_old_root,
        utxo2_new_root,
    );
    let agg_utxo2_proof = prove_and_verify(&agg_utxo2).await.unwrap();

    let agg_agg = AggAgg::new([
        AggProof::AggUtxo(Box::new(agg_utxo1_proof)),
        AggProof::AggUtxo(Box::new(agg_utxo2_proof)),
    ]);
    let agg_agg_proof = prove_and_verify(&agg_agg).await.unwrap();

    let agg_final = AggFinal::new(agg_agg_proof);

    prove_and_verify(&agg_final).await.unwrap();
}

#[tokio::test]
async fn test_utxo_mint() {
    let (_secret_key, address) = get_keypair(101);

    let input_note1 = InputNote::padding_note();
    let input_note2 = InputNote::padding_note();

    let output_note1 = send_note(100, address, 1);
    let output_note2 = send_note(50, address, 2);

    let utxo = Utxo {
        input_notes: [input_note1, input_note2],
        output_notes: [output_note1, output_note2],
        kind: UtxoKind::Mint,
        burn_address: None,
    };

    prove_and_verify(&utxo).await.unwrap();
}

#[tokio::test]
async fn test_utxo_burn() {
    let (secret_key, address) = get_keypair(101);
    let burn_address = Element::new(999);

    let input_note1 = InputNote {
        note: send_note(80, address, 1),
        secret_key,
    };

    let input_note2 = InputNote {
        note: send_note(70, address, 2),
        secret_key,
    };

    let output_note1 = send_note(20, address, 3);
    let output_note2 = send_note(30, address, 4);

    let utxo = Utxo {
        input_notes: [input_note1, input_note2],
        output_notes: [output_note1, output_note2],
        kind: UtxoKind::Burn,
        burn_address: Some(burn_address),
    };

    prove_and_verify(&utxo).await.unwrap();
}

#[tokio::test]
async fn test_agg_utxo_mint_burn() {
    let (secret_key, address) = get_keypair(101);
    let mut tree = smirk::Tree::<161, ()>::new();

    let utxo1_input_note1 = InputNote {
        note: send_note(50, address, 1),
        secret_key,
    };
    add_note_to_tree(&mut tree, &utxo1_input_note1);

    let utxo1_input_note2 = InputNote {
        note: send_note(30, address, 2),
        secret_key,
    };
    add_note_to_tree(&mut tree, &utxo1_input_note2);

    let utxo1_old_root = tree.root_hash();

    let utxo1_output_note1 = send_note(40, address, 3);
    let utxo1_output_note2 = send_note(40, address, 4);

    let utxo1 = Utxo {
        input_notes: [utxo1_input_note1.clone(), utxo1_input_note2.clone()],
        output_notes: [utxo1_output_note1.clone(), utxo1_output_note2.clone()],
        kind: UtxoKind::Send,
        burn_address: None,
    };

    let (utxo1_proof, p1, p2, p3, p4, _) = process_utxo_for_agg(&mut tree, &utxo1).await.unwrap();
    verify_utxo_proof(&utxo1_proof).await;

    // Second UTXO: Burn
    let utxo2_input_note1 = InputNote {
        note: utxo1_output_note1.clone(),
        secret_key,
    };

    let utxo2 = Utxo {
        input_notes: [utxo2_input_note1.clone(), InputNote::padding_note()],
        output_notes: [Note::padding_note(), Note::padding_note()],
        kind: UtxoKind::Burn,
        burn_address: Some(Element::new(999)),
    };

    let (utxo2_proof, p2_1, p2_2, p2_3, p2_4, _) =
        process_utxo_for_agg(&mut tree, &utxo2).await.unwrap();
    verify_utxo_proof(&utxo2_proof).await;

    // Third UTXO: Mint
    let utxo3_output_note1 = send_note(100, address, 1);

    let utxo3 = Utxo {
        input_notes: [InputNote::padding_note(), InputNote::padding_note()],
        output_notes: [utxo3_output_note1.clone(), Note::padding_note()],
        kind: UtxoKind::Mint,
        burn_address: None,
    };

    let (utxo3_proof, p3_1, p3_2, p3_3, p3_4, utxo3_new_root) =
        process_utxo_for_agg(&mut tree, &utxo3).await.unwrap();
    verify_utxo_proof(&utxo3_proof).await;

    let agg_utxo1 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo1_proof, &[p1, p2, p3, p4]),
            UtxoProofBundleWithMerkleProofs::new(utxo2_proof, &[p2_1, p2_2, p2_3, p2_4]),
            UtxoProofBundleWithMerkleProofs::new(utxo3_proof, &[p3_1, p3_2, p3_3, p3_4]),
        ],
        utxo1_old_root,
        utxo3_new_root,
    );

    prove_and_verify(&agg_utxo1).await.unwrap();
}

#[tokio::test]
async fn test_agg_final_with_mint_and_burn() {
    let (secret_key, address) = get_keypair(101);
    let mut tree = smirk::Tree::<161, ()>::new();

    let utxo1_input_note1 = InputNote {
        note: send_note(50, address, 1),
        secret_key,
    };
    add_note_to_tree(&mut tree, &utxo1_input_note1);

    let utxo1_input_note2 = InputNote {
        note: send_note(30, address, 2),
        secret_key,
    };
    add_note_to_tree(&mut tree, &utxo1_input_note2);

    let utxo1_old_root = tree.root_hash();

    let utxo1_output_note1 = send_note(40, address, 3);
    let utxo1_output_note2 = send_note(40, address, 4);

    let utxo1 = Utxo {
        input_notes: [utxo1_input_note1.clone(), utxo1_input_note2.clone()],
        output_notes: [utxo1_output_note1.clone(), utxo1_output_note2.clone()],
        kind: UtxoKind::Send,
        burn_address: None,
    };

    let (utxo1_proof, p1, p2, p3, p4, _) = process_utxo_for_agg(&mut tree, &utxo1).await.unwrap();
    verify_utxo_proof(&utxo1_proof).await;

    // Second UTXO: Burn
    let utxo2_input_note1 = InputNote {
        note: utxo1_output_note1.clone(),
        secret_key,
    };

    let utxo2 = Utxo {
        input_notes: [utxo2_input_note1.clone(), InputNote::padding_note()],
        output_notes: [Note::padding_note(), Note::padding_note()],
        kind: UtxoKind::Burn,
        burn_address: Some(Element::new(999)),
    };

    let (utxo2_proof, p2_1, p2_2, p2_3, p2_4, utxo2_new_root) =
        process_utxo_for_agg(&mut tree, &utxo2).await.unwrap();
    verify_utxo_proof(&utxo2_proof).await;

    let agg_utxo1 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo1_proof, &[p1, p2, p3, p4]),
            UtxoProofBundleWithMerkleProofs::new(utxo2_proof, &[p2_1, p2_2, p2_3, p2_4]),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo1_old_root,
        utxo2_new_root,
    );
    let agg_utxo1_proof = prove_and_verify(&agg_utxo1).await.unwrap();

    // Third UTXO: Mint
    let utxo3_output_note1 = send_note(100, address, 1);

    let utxo3 = Utxo {
        input_notes: [InputNote::padding_note(), InputNote::padding_note()],
        output_notes: [utxo3_output_note1.clone(), Note::padding_note()],
        kind: UtxoKind::Mint,
        burn_address: None,
    };

    let (utxo3_proof, p3_1, p3_2, p3_3, p3_4, utxo3_new_root) =
        process_utxo_for_agg(&mut tree, &utxo3).await.unwrap();
    verify_utxo_proof(&utxo3_proof).await;

    let agg_utxo2 = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo3_proof, &[p3_1, p3_2, p3_3, p3_4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        utxo2_new_root,
        utxo3_new_root,
    );
    let agg_utxo2_proof = prove_and_verify(&agg_utxo2).await.unwrap();

    // Create AggAgg with both mint and burn proofs
    let agg_agg = AggAgg::new([
        AggProof::AggUtxo(Box::new(agg_utxo1_proof)),
        AggProof::AggUtxo(Box::new(agg_utxo2_proof)),
    ]);
    let agg_agg_proof = prove_and_verify(&agg_agg).await.unwrap();

    // Create AggFinal
    let agg_final = AggFinal::new(agg_agg_proof);
    prove_and_verify(&agg_final).await.unwrap();
}
