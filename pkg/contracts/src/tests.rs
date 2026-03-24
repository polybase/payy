// lint-long-file-override allow-max-lines=1000
#[cfg(test)]
mod test_rollup;

use crate::util::{calculate_domain_separator, convert_element_to_h256, convert_h160_to_element};
use barretenberg_cli::CliBackend;
use element::Element;
use ethereum_types::{H256, U256};
use hash::hash_merge;
use secp256k1::PublicKey;
use std::str::FromStr;
use std::sync::Arc;
use test_rollup::rollup::Rollup;
use testutil::ACCOUNT_1_SK;
use testutil::eth::{EthNode, EthNodeOptions};
use web3::signing::{SecretKey, keccak256};
use web3::types::Address;
use zk_circuits::{Proof, Prove};
use zk_primitives::{
    AggAgg, AggProof, AggUtxo, AggUtxoProof, InputNote, MerklePath, Note, Utxo, UtxoKind,
    UtxoProof, UtxoProofBundleWithMerkleProofs, bridged_polygon_usdc_note_kind,
    get_address_for_private_key,
};
// use zk_circuits::constants::MERKLE_TREE_DEPTH;
// use zk_circuits::data::{BurnTo, Mint, ParameterSet};
// use zk_circuits::test::rollup::Rollup;

use super::*;

struct Env {
    _eth_node: Arc<EthNode>,
    evm_secret_key: SecretKey,
    evm_address: Address,
    rollup_contract: RollupContract,
    usdc_contract: USDCContract,
}

async fn make_env(options: EthNodeOptions) -> Env {
    let eth_node = EthNode::new(options).run_and_deploy().await;

    let evm_secret_key = SecretKey::from_str(ACCOUNT_1_SK).unwrap();
    let evm_address = to_address(&evm_secret_key);

    let rollup_contract = RollupContract::from_eth_node(&eth_node, evm_secret_key)
        .await
        .unwrap();
    let usdc_contract = USDCContract::from_eth_node(&eth_node, evm_secret_key)
        .await
        .unwrap();

    Env {
        _eth_node: eth_node,
        evm_secret_key,
        evm_address,
        rollup_contract,
        usdc_contract,
    }
}

fn to_address(secret_key: &SecretKey) -> Address {
    let secret_key_bytes = secret_key.secret_bytes();
    let secp = secp256k1::Secp256k1::new();
    let secret_key = secp256k1::SecretKey::from_slice(&secret_key_bytes).unwrap();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let serialized_public_key = public_key.serialize_uncompressed();

    // Ethereum address is the last 20 bytes of the Keccak hash of the public key
    let address_bytes = &keccak256(&serialized_public_key[1..])[12..];
    Address::from_slice(address_bytes)
}

async fn sign_block(new_root: &Element, height: u64, other_hash: [u8; 32]) -> Vec<u8> {
    let env = make_env(EthNodeOptions::default()).await;

    let proposal_hash = keccak256(&{
        let mut bytes = vec![];
        bytes.extend_from_slice(convert_element_to_h256(new_root).as_bytes());

        let mut height_bytes = [0u8; 32];
        U256::from(height).to_big_endian(&mut height_bytes);
        bytes.extend_from_slice(&height_bytes);

        bytes.extend_from_slice(&other_hash);
        bytes
    });

    let accept_hash = keccak256(&{
        let mut bytes = vec![];

        let mut height_bytes = [0u8; 32];
        U256::from(height + 1).to_big_endian(&mut height_bytes);
        bytes.extend_from_slice(&height_bytes);

        bytes.extend_from_slice(&proposal_hash);

        bytes
    });

    let msg = keccak256(&{
        let mut bytes = vec![];
        bytes.extend_from_slice(&("Payy".len() as u64).to_be_bytes());
        bytes.extend_from_slice(b"Payy");
        bytes.extend_from_slice(&accept_hash);
        bytes
    });

    let sig = secp256k1::SECP256K1.sign_ecdsa_recoverable(
        &secp256k1::Message::from_digest(msg),
        &secp256k1::SecretKey::from_slice(&env.evm_secret_key.secret_bytes()).unwrap(),
    );
    let (recovery, r_s) = sig.serialize_compact();
    let mut sig = vec![0u8; 65];
    sig[0..64].copy_from_slice(&r_s[0..64]);
    sig[64] = recovery.to_i32() as u8;
    sig
}

#[tokio::test]
async fn root_hash() {
    let env = make_env(EthNodeOptions::default()).await;

    let _root_hash = env.rollup_contract.root_hash().await.unwrap();
}

#[tokio::test]
async fn height() {
    let env = make_env(EthNodeOptions::default()).await;

    let _height = env.rollup_contract.block_height().await.unwrap();
}

pub fn get_keypair(key: u64) -> (Element, Element) {
    let secret_key = Element::new(key);
    let address = get_address_for_private_key(secret_key);
    (secret_key, address)
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

pub fn send_note(value: u64, address: Element, psi: u64) -> Note {
    note(value, address, psi, bridged_polygon_usdc_note_kind())
}

pub fn bb_backend() -> CliBackend {
    CliBackend
}

pub async fn verify_proof(proof: &impl zk_circuits::Verify) {
    let bb_backend = bb_backend();
    let start = std::time::Instant::now();
    let result = proof.verify(&bb_backend).await;
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Proof verification failed: {:?}",
        result.err()
    );

    println!("Proof verification completed in {duration:?}");
}

pub async fn verify_utxo_proof(proof: &UtxoProof) {
    verify_proof(&Proof::from(proof.clone())).await;
}

pub async fn prove_and_verify<P: Prove>(proof_input: &P) -> Result<P::Proof> {
    let proof = prove_proof(proof_input).await?;
    verify_proof(&proof).await;
    Ok(proof)
}

pub async fn prove_proof<P: Prove>(proof_input: &P) -> Result<P::Proof> {
    let bb_backend = bb_backend();
    let start = std::time::Instant::now();
    let proof = proof_input.prove(&bb_backend).await.unwrap();
    let end = std::time::Instant::now() - start;
    println!("Proving completed in {end:?}");
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
    let bb_backend = bb_backend();
    let utxo_proof = utxo.clone().prove(&bb_backend).await.unwrap();

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

#[tokio::test]
async fn verify_transfers() {
    let env = make_env(EthNodeOptions::default()).await;

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

    // Create final aggregation proof
    let agg_final = zk_primitives::AggFinal::new(agg_agg_proof);
    let agg_final_proof = prove_and_verify(&agg_final).await.unwrap();

    // Sign
    let other_hash = [0u8; 32];
    let height = 1;
    let sig = sign_block(&agg_agg.new_root(), height, other_hash).await;

    // Set the root, we add some pre-existing values to the tree before generating the UTXO,
    // so the tree is not empty
    env.rollup_contract
        .set_root(&agg_agg.old_root())
        .await
        .unwrap();

    assert_eq!(agg_final_proof.proof.0.len(), 330 * 32);
    env.rollup_contract
        .verify_block(
            &agg_final_proof.proof.0,
            &agg_agg.old_root(),
            &agg_agg.new_root(),
            &agg_agg.commit_hash(),
            &agg_final_proof.public_inputs.messages,
            &agg_final_proof.kzg,
            other_hash,
            height,
            &[&sig],
            500_000,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn mint_with_authorization() {
    let env = make_env(EthNodeOptions::default()).await;
    let rollup = Rollup::new();
    let bob = rollup.new_wallet();

    let amount = 10 * 10u64.pow(6);
    let note = bob.new_note(amount, bridged_polygon_usdc_note_kind());

    let secret_key = secp256k1::SecretKey::from_slice(&env.evm_secret_key.secret_bytes()).unwrap();

    let nonce = Element::secure_random(rand::thread_rng());
    let valid_after = U256::from(0);
    let valid_before = U256::from(u64::MAX);

    let mint_hash = hash_merge([note.psi, Note::padding_note().psi]);

    // Sig for the USDC function
    let sig_bytes = env.usdc_contract.signature_for_receive(
        env.evm_address,
        env.rollup_contract.address(),
        amount.into(),
        valid_after,
        valid_before,
        H256::from(nonce.to_be_bytes()),
        secret_key,
    );

    // Sig for our mint function
    let mint_sig_bytes = env.rollup_contract.signature_for_mint(
        mint_hash,
        amount.into(),
        note.contract,
        env.evm_address,
        valid_after,
        valid_before,
        H256::from(nonce.to_be_bytes()),
        secret_key,
    );

    env.rollup_contract
        .mint_with_authorization(
            &mint_hash,
            &note.value,
            &note.contract,
            &env.evm_address,
            U256::from(0),
            U256::from(u64::MAX),
            H256::from(nonce.to_be_bytes()),
            &sig_bytes,
            &mint_sig_bytes,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn mint_from() {
    let env = make_env(EthNodeOptions::default()).await;
    let rollup = Rollup::new();
    let bob = rollup.new_wallet();

    // Create the proof
    let note = bob.new_note(10 * 10u64.pow(6), bridged_polygon_usdc_note_kind());

    let mint_hash = hash_merge([note.psi, Note::padding_note().psi]);

    env.usdc_contract
        .approve_max(env.rollup_contract.address())
        .await
        .unwrap();

    env.rollup_contract
        .mint(&mint_hash, &note.value, &note.contract)
        .await
        .unwrap();
}

#[tokio::test]
async fn burn_to() {
    // Set up the environment
    let env = make_env(EthNodeOptions::default()).await;

    env.usdc_contract
        .transfer(env.rollup_contract.address(), 100)
        .await
        .unwrap();

    let (secret_key, address) = get_keypair(101);
    let mut tree = smirk::Tree::<161, ()>::new();

    // Create input note for burning
    let input_note1 = InputNote {
        note: send_note(100, address, 1),
        secret_key,
    };
    tree.insert(input_note1.note.commitment(), ()).unwrap();

    // Add a padding input note
    let input_note2 = InputNote::padding_note();

    let old_root = tree.root_hash();

    // Create burn UTXO
    let burn_address = convert_h160_to_element(&env.evm_address);
    let utxo = Utxo::new_burn([input_note1.clone(), input_note2.clone()], burn_address);

    let backend = bb_backend();

    // Generate UTXO proof
    let utxo_proof = utxo.prove(&backend).await.unwrap();

    // Get merkle paths for inputs before removal
    let p1 = MerklePath::new(
        tree.path_for(input_note1.note.commitment())
            .siblings
            .to_vec(),
    );
    let p2 = MerklePath::new(
        tree.path_for(input_note2.note.commitment())
            .siblings
            .to_vec(),
    );

    // Remove input notes (only the first one, as the second is padding)
    tree.remove(input_note1.note.commitment()).unwrap();

    // For outputs, both are padding notes with commitment zero
    let output_commit = Element::ZERO;
    let p3 = MerklePath::new(tree.path_for(output_commit).siblings.to_vec());
    let p4 = MerklePath::new(tree.path_for(output_commit).siblings.to_vec());

    let new_root = tree.root_hash();

    // Create an AggUtxo with the burn UTXO and padding
    let agg_utxo = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo_proof, &[p1, p2, p3, p4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        old_root,
        new_root,
    );

    // Prove the AggUtxo
    let agg_utxo_proof = agg_utxo.prove(&backend).await.unwrap();

    // Create a padding AggUtxo for AggAgg
    let padding_agg_utxo_proof = AggUtxoProof::default();

    // Create and prove the AggAgg
    let agg_agg = AggAgg::new([
        AggProof::AggUtxo(Box::new(agg_utxo_proof)),
        AggProof::AggUtxo(Box::new(padding_agg_utxo_proof)),
    ]);
    let agg_agg_proof = agg_agg.prove(&backend).await.unwrap();

    // Create final aggregation proof
    let agg_final = zk_primitives::AggFinal::new(agg_agg_proof);
    let agg_final_proof = prove_and_verify(&agg_final).await.unwrap();

    // Sign the block
    let other_hash = [0u8; 32];
    let height = 1;
    let sig = sign_block(&agg_agg.new_root(), height, other_hash).await;

    // Get the initial balance of the EVM address
    let initial_balance = env.usdc_contract.balance(env.evm_address).await.unwrap();

    env.rollup_contract
        .set_root(&agg_agg.old_root())
        .await
        .unwrap();

    // Submit the proof to the contract
    env.rollup_contract
        .verify_block(
            &agg_final_proof.proof.0,
            &agg_agg.old_root(),
            &agg_agg.new_root(),
            &agg_agg.commit_hash(),
            &agg_final_proof.public_inputs.messages,
            &agg_final_proof.kzg,
            other_hash,
            height,
            &[&sig],
            500_000,
        )
        .await
        .unwrap();

    // Verify the balance increased by the burnt amount
    let new_balance = env.usdc_contract.balance(env.evm_address).await.unwrap();
    let burnt_value = U256::from(100);
    assert_eq!(new_balance, initial_balance + burnt_value);
}

#[tokio::test]
async fn substitute_burn() {
    // Set up the environment
    let env = make_env(EthNodeOptions::default()).await;

    env.usdc_contract
        .transfer(env.rollup_contract.address(), 100)
        .await
        .unwrap();

    let (secret_key, address) = get_keypair(101);
    let mut tree = smirk::Tree::<161, ()>::new();

    // Create input note for burning
    let input_note1 = InputNote {
        note: send_note(100, address, 1),
        secret_key,
    };
    tree.insert(input_note1.note.commitment(), ()).unwrap();

    // Add a padding input note
    let input_note2 = InputNote::padding_note();

    let old_root = tree.root_hash();

    // Create burn UTXO
    let burn_address = Address::random();
    let utxo = Utxo::new_burn(
        [input_note1.clone(), input_note2.clone()],
        convert_h160_to_element(&burn_address),
    );

    let hash = utxo.input_notes[0].note.commitment();

    let backend = bb_backend();

    // Generate UTXO proof
    let utxo_proof = utxo.prove(&backend).await.unwrap();

    // Get merkle paths for inputs before removal
    let p1 = MerklePath::new(
        tree.path_for(input_note1.note.commitment())
            .siblings
            .to_vec(),
    );
    let p2 = MerklePath::new(
        tree.path_for(input_note2.note.commitment())
            .siblings
            .to_vec(),
    );

    // Remove input notes (only the first one, as the second is padding)
    tree.remove(input_note1.note.commitment()).unwrap();

    // For outputs, both are padding notes with commitment zero
    let output_commit = Element::ZERO;
    let p3 = MerklePath::new(tree.path_for(output_commit).siblings.to_vec());
    let p4 = MerklePath::new(tree.path_for(output_commit).siblings.to_vec());

    let new_root = tree.root_hash();

    // Create an AggUtxo with the burn UTXO and padding
    let agg_utxo = AggUtxo::new(
        [
            UtxoProofBundleWithMerkleProofs::new(utxo_proof, &[p1, p2, p3, p4]),
            UtxoProofBundleWithMerkleProofs::default(),
            UtxoProofBundleWithMerkleProofs::default(),
        ],
        old_root,
        new_root,
    );

    // Prove the AggUtxo
    let agg_utxo_proof = agg_utxo.prove(&backend).await.unwrap();

    // Create a padding AggUtxo for AggAgg
    let padding_agg_utxo_proof = AggUtxoProof::default();

    // Create and prove the AggAgg
    let agg_agg = AggAgg::new([
        AggProof::AggUtxo(Box::new(agg_utxo_proof)),
        AggProof::AggUtxo(Box::new(padding_agg_utxo_proof)),
    ]);
    let agg_agg_proof = agg_agg.prove(&backend).await.unwrap();

    // Create final aggregation proof
    let agg_final = zk_primitives::AggFinal::new(agg_agg_proof);
    let agg_final_proof = prove_and_verify(&agg_final).await.unwrap();

    // Sign the block
    let other_hash = [0u8; 32];
    let height = 1;
    let sig = sign_block(&agg_agg.new_root(), height, other_hash).await;

    // Get the initial balance of the EVM address
    let initial_caller_balance = env.usdc_contract.balance(env.evm_address).await.unwrap();
    let initial_burn_address_balance = env.usdc_contract.balance(burn_address).await.unwrap();

    env.rollup_contract
        .set_root(&agg_agg.old_root())
        .await
        .unwrap();

    env.rollup_contract
        .substitute_burn(
            &burn_address,
            &input_note1.note.contract,
            &hash,
            &Element::new(100),
            height,
        )
        .await
        .unwrap();

    let balance_after_substitute = env.usdc_contract.balance(env.evm_address).await.unwrap();
    assert_eq!(balance_after_substitute, initial_caller_balance - 100);

    // Submit the proof to the contract
    env.rollup_contract
        .verify_block(
            &agg_final_proof.proof.0,
            &agg_agg.old_root(),
            &agg_agg.new_root(),
            &agg_agg.commit_hash(),
            &agg_final_proof.public_inputs.messages,
            &agg_final_proof.kzg,
            other_hash,
            height,
            &[&sig],
            500_000,
        )
        .await
        .unwrap();

    // Verify the balance increased by the burnt amount
    let new_balance = env.usdc_contract.balance(burn_address).await.unwrap();
    let burnt_value = U256::from(100);
    assert_eq!(new_balance, initial_burn_address_balance + burnt_value);

    let new_caller_balance = env.usdc_contract.balance(env.evm_address).await.unwrap();
    assert_eq!(new_caller_balance, initial_caller_balance);

    let was_substituted = env
        .rollup_contract
        .was_burn_substituted(
            &burn_address,
            &input_note1.note.contract,
            &hash,
            &Element::new(100),
        )
        .await
        .unwrap();
    assert!(!was_substituted);
}

// #[tokio::test]
// async fn substitute_burn() {
//     let env = make_env(EthNodeOptions {
//         use_noop_verifier: true,
//         ..Default::default()
//     })
//     .await;

//     // Create the proof
//     let mut rollup = Rollup::new();
//     let bob = rollup.new_wallet();

//     let bob_note = rollup.unverified_add_unspent_note(&bob, 100);

//     // Set the root, we add some pre-existing values to the tree before generating the UTXO,
// #[tokio::test]
// async fn set_validators() {
//     let env = make_env(EthNodeOptions::default()).await;

//     // let's also test the worker
//     let worker_rollup_contract = env.rollup_contract.clone();
//     let _worker = tokio::spawn(async move {
//         worker_rollup_contract
//             .worker(Duration::from_millis(100))
//             .await
//     });

//     let validator_sets_before = env.rollup_contract.get_validator_sets(0).await.unwrap();
//     assert_eq!(
//         validator_sets_before,
//         *env.rollup_contract.validator_sets.read()
//     );

//     let valid_from = validator_sets_before.last().unwrap().valid_from + 2;
//     let tx = env
//         .rollup_contract
//         .set_validators(valid_from.as_u64(), &[env.evm_address])
//         .await
//         .unwrap();

//     // Wait for receipt
//     while env
//         .rollup_contract
//         .client
//         .client()
//         .eth()
//         .transaction_receipt(tx)
//         .await
//         .unwrap()
//         .is_none()
//     {
//         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//     }

//     let validator_sets_after = env
//         .rollup_contract
//         .get_validator_sets(validator_sets_before.len() as u64)
//         .await
//         .unwrap();
//     assert_eq!(validator_sets_after.last().unwrap().valid_from, valid_from);
//     assert_eq!(
//         validator_sets_after.last().unwrap().validators,
//         vec![env.evm_address]
//     );

//     // Wait for worker to update the validator sets
//     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//     // Make sure the worker updated the contract's state
//     assert_eq!(
//         validator_sets_before
//             .into_iter()
//             .chain(validator_sets_after)
//             .collect::<Vec<_>>(),
//         *env.rollup_contract.validator_sets.read()
//     );
// }

#[test]
fn empty_root() {
    let tree = smirk::Tree::<161, ()>::new();
    let hash = expect_test::expect_file!["./empty_merkle_tree_root_hash.txt"];
    hash.assert_eq(&tree.root_hash().to_hex());
}

#[test]
fn test_domain_separator_calculation() {
    // Test primary rollup chain values (Chain ID: 137 - Polygon)
    let chain_id = U256::from(137);

    // Test USDC contract
    let usdc_address: Address = "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359"
        .parse()
        .unwrap();
    let usdc_domain_separator = calculate_domain_separator("USD Coin", "2", chain_id, usdc_address);
    let expected_usdc = H256::from_slice(
        &hex::decode("caa2ce1a5703ccbe253a34eb3166df60a705c561b44b192061e28f2a985be2ca").unwrap(),
    );
    assert_eq!(
        usdc_domain_separator, expected_usdc,
        "USDC domain separator mismatch"
    );

    // Test AcrossWithAuthorization contract
    let across_address: Address = "0xf5bf1a6a83029503157bb3761488bb75d64002e7"
        .parse()
        .unwrap();
    let across_domain_separator =
        calculate_domain_separator("AcrossWithAuthorization", "1", chain_id, across_address);
    let expected_across = H256::from_slice(
        &hex::decode("c0db9d13ac268c870ccb743fd1078a25b4c98ff3ba232167b02aff4340f8c8cc").unwrap(),
    );
    assert_eq!(
        across_domain_separator, expected_across,
        "AcrossWithAuthorization domain separator mismatch"
    );

    // Test Rollup contract
    let rollup_address: Address = "0xcd92281548df923141fd9b690c7c8522e12e76e6"
        .parse()
        .unwrap();
    let rollup_domain_separator =
        calculate_domain_separator("Rollup", "1", chain_id, rollup_address);
    let expected_rollup = H256::from_slice(
        &hex::decode("5261b2c944771285325623d865717567b4425028487b653028b59e46d910b34d").unwrap(),
    );
    assert_eq!(
        rollup_domain_separator, expected_rollup,
        "Rollup domain separator mismatch"
    );
}
