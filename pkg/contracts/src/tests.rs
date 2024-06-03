use crate::util::{convert_element_to_h256, convert_h160_to_element};
use secp256k1::rand::random;
use secp256k1::PublicKey;
use smirk::Element;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use testutil::eth::EthNode;
use web3::signing::{keccak256, SecretKey};
use web3::types::Address;
use zk_circuits::constants::MERKLE_TREE_DEPTH;
use zk_circuits::data::{Burn, Mint, ParameterSet};
use zk_circuits::test::rollup::Rollup;

use super::*;

const ACCOUNT_1_SK: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

pub struct Env {
    _eth_node: Arc<EthNode>,
    pub evm_secret_key: SecretKey,
    pub evm_address: Address,
    pub rollup_contract: RollupContract,
    pub rollup_contract_addr: Address,
    pub usdc_contract: USDCContract,
    // client: Client,
}

pub async fn make_env() -> Env {
    let eth_node = EthNode::run_and_deploy().await;

    let rpc = std::env::var("ETHEREUM_RPC").unwrap_or(eth_node.rpc_url());

    let rollup_addr = std::env::var("ROLLUP_CONTRACT_ADDR")
        .unwrap_or("2279b7a0a67db372996a5fab50d91eaa73d2ebe6".to_string());

    let usdc_addr = &std::env::var("USDC_CONTRACT_ADDR")
        .unwrap_or("5fbdb2315678afecb367f032d93f642f64180aa3".to_string());

    let evm_secret_key = SecretKey::from_str(&std::env::var("PROVER_SECRET_KEY").unwrap_or(
        // Seems to be the default when deploying with hardhat to a local node
        ACCOUNT_1_SK.to_owned(),
    ))
    .unwrap();

    let evm_address = to_address(&evm_secret_key);

    let client = Client::new(&rpc, None);

    Env {
        _eth_node: eth_node,
        evm_secret_key,
        evm_address,
        rollup_contract: RollupContract::load(client.clone(), &rollup_addr, evm_secret_key)
            .await
            .unwrap(),
        rollup_contract_addr: Address::from_str(&rollup_addr).unwrap(),
        usdc_contract: USDCContract::load(client, usdc_addr, evm_secret_key)
            .await
            .unwrap(),
        // client,
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
    let env = make_env().await;

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
        bytes.extend_from_slice(&("Polybase".len() as u64).to_be_bytes());
        bytes.extend_from_slice(b"Polybase");
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
async fn root_hashes() {
    let env: Env = make_env().await;

    let _root_hashes = env.rollup_contract.root_hashes().await.unwrap();
}

#[tokio::test]
async fn root_hash() {
    let env = make_env().await;

    let _root_hash = env.rollup_contract.root_hash().await.unwrap();
}

#[tokio::test]
async fn height() {
    let env = make_env().await;

    let _height = env.rollup_contract.block_height().await.unwrap();
}

#[tokio::test]

async fn verify_transfers() {
    let env = make_env().await;
    let params_21 = zk_circuits::data::ParameterSet::TwentyOne;

    let utxo_aggs = zk_circuits::test::agg_utxo::create_or_load_agg_utxo_snarks(params_21);

    let aggregate_agg =
        zk_circuits::test::agg_agg::create_or_load_agg_agg_utxo_snark(params_21, utxo_aggs);

    let aggregate_agg_agg = zk_circuits::test::agg_agg::create_or_load_agg_agg_final_evm_proof(
        params_21,
        aggregate_agg,
    )
    .try_as_v_1()
    .unwrap();

    // Public inputs
    let agg_instances = aggregate_agg_agg.agg_instances;
    let agg_instances: Vec<_> = agg_instances.into_iter().map(From::from).collect();
    let old_root = aggregate_agg_agg.old_root;
    let new_root = aggregate_agg_agg.new_root;
    let utxo_inputs = aggregate_agg_agg.utxo_inputs;
    let proof = aggregate_agg_agg.proof;

    // Sign
    let other_hash = [0u8; 32];
    let height = 1;
    let sig = sign_block(&new_root, height, other_hash).await;

    // Set the root, we add some pre-existing values to the tree before generating the UTXO,
    // so the tree is not empty
    env.rollup_contract.set_root(&old_root).await.unwrap();

    env.rollup_contract
        .verify_block(
            &proof,
            agg_instances.try_into().unwrap(),
            &old_root,
            &new_root,
            &utxo_inputs,
            other_hash,
            height,
            &[&sig],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn mint_from() {
    let env = make_env().await;
    let rollup = Rollup::new();
    let bob = rollup.new_wallet();

    // Create the proof
    let note = bob.new_note(10 * 10u64.pow(6));
    let mint = Mint::new([note.clone()]);
    let params = ParameterSet::Eight;
    let proof = mint.evm_proof(params).unwrap();

    env.usdc_contract
        .approve_max(env.rollup_contract_addr)
        .await
        .unwrap();

    env.rollup_contract
        .mint(&proof, &note.commitment(), &note.value(), &note.source())
        .await
        .unwrap();
}

#[tokio::test]
async fn mint_with_authorization() {
    let env = make_env().await;
    let rollup = Rollup::new();
    let bob = rollup.new_wallet();

    let amount = 10 * 10u64.pow(6);
    let note = bob.new_note(amount);
    let mint = Mint::new([note.clone()]);
    let params = ParameterSet::Eight;
    let proof = mint.evm_proof(params).unwrap();

    let secret_key = secp256k1::SecretKey::from_slice(&env.evm_secret_key.secret_bytes()).unwrap();

    let nonce = random();
    let valid_after = U256::from(0);
    let valid_before = U256::from(u64::MAX);

    // Sig for the USDC function
    let sig_bytes = env.usdc_contract.signature_for_receive(
        env.evm_address,
        env.rollup_contract_addr,
        amount.into(),
        valid_after,
        valid_before,
        nonce,
        secret_key,
    );

    // Sig for our mint function
    let mint_sig_bytes = env.rollup_contract.signature_for_mint(
        note.commitment(),
        amount.into(),
        note.source(),
        env.evm_address,
        valid_after,
        valid_before,
        nonce,
        secret_key,
    );

    env.rollup_contract
        .mint_with_authorization(
            &proof,
            &note.commitment(),
            &note.value(),
            &note.source(),
            &env.evm_address,
            U256::from(0),
            U256::from(u64::MAX),
            nonce,
            &sig_bytes,
            &mint_sig_bytes,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn burn() {
    let env = make_env().await;

    // Create the proof
    let mut rollup = Rollup::new();
    let bob = rollup.new_wallet();

    let bob_note = rollup.unverified_add_unspent_note(&bob, 100);

    // Set the root, we add some pre-existing values to the tree before generating the UTXO,
    // so the tree is not empty
    env.rollup_contract
        .set_root(&rollup.root_hash())
        .await
        .unwrap();

    let note = bob_note.note();
    let burn = Burn {
        notes: [note.clone()],
        secret_key: bob.pk,
        to_address: convert_h160_to_element(&env.evm_address),
    };

    let proof = burn.evm_proof(ParameterSet::Nine).unwrap();

    env.rollup_contract
        .burn(
            // User1 address (where we will send the burned funds)
            &env.evm_address,
            &proof,
            &note.nullifier(bob.pk),
            &note.value(),
            &note.source(),
            &burn.signature(&note),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn set_validators() {
    let env = make_env().await;

    // let's also test the worker
    let worker_rollup_contract = env.rollup_contract.clone();
    let _worker = tokio::spawn(async move {
        worker_rollup_contract
            .worker(Duration::from_millis(100))
            .await
    });

    let validator_sets_before = env.rollup_contract.get_validator_sets(0).await.unwrap();
    assert_eq!(
        validator_sets_before,
        *env.rollup_contract.validator_sets.read()
    );

    let valid_from = validator_sets_before.last().unwrap().valid_from + 2;
    let tx = env
        .rollup_contract
        .set_validators(valid_from.as_u64(), &[env.evm_address])
        .await
        .unwrap();

    // Wait for receipt
    while env
        .rollup_contract
        .client
        .client()
        .eth()
        .transaction_receipt(tx)
        .await
        .unwrap()
        .is_none()
    {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    let validator_sets_after = env
        .rollup_contract
        .get_validator_sets(validator_sets_before.len() as u64)
        .await
        .unwrap();
    assert_eq!(validator_sets_after.last().unwrap().valid_from, valid_from);
    assert_eq!(
        validator_sets_after.last().unwrap().validators,
        vec![env.evm_address]
    );

    // Wait for worker to update the validator sets
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    // Make sure the worker updated the contract's state
    assert_eq!(
        validator_sets_before
            .into_iter()
            .chain(validator_sets_after)
            .collect::<Vec<_>>(),
        *env.rollup_contract.validator_sets.read()
    );
}

#[test]
fn empty_root() {
    let tree = smirk::Tree::<MERKLE_TREE_DEPTH, ()>::new();
    let hash = expect_test::expect_file!["./empty_merkle_tree_root_hash.txt"];
    hash.assert_eq(format!("{:?}", tree.root_hash().to_base()).as_str());
}
