// lint-long-file-override allow-max-lines=300
use crate::circuits::generated::erc20_transfer::{Erc20TransferInput, Erc20transfer, Note};
use crate::circuits::generated::submodules::common::Merklepath;
use crate::circuits::generated::submodules::signature::Signature as SignatureStruct;
use crate::circuits::{note_v2::hash_note, tests::prove_and_verify};

use alloy::consensus::{SignableTransaction, TxEip1559};
use alloy::primitives::{Address, Bytes, U256};
use alloy::signers::Signer;
use alloy::signers::local::PrivateKeySigner;
use bitvec::{array::BitArray, order::Lsb0};
use element::Element;
use sha3::{Digest, Keccak256};
use std::str::FromStr;

fn build_erc20_transfer_calldata(receiver: [u8; 20], amount: u64) -> Bytes {
    let mut data = Vec::with_capacity(68);
    // transfer(address,uint256) selector
    data.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]);
    // receiver address (padded to 32 bytes)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&receiver);
    data.extend_from_slice(&U256::from(amount).to_be_bytes::<32>());
    Bytes::from(data)
}

/// Helper function to build Merkle proofs for two commitments
/// Returns (root, [path_for_commitment_1, path_for_commitment_2])
fn build_merkle_inputs(
    commitment_one: Element,
    commitment_two: Element,
) -> (Element, [[Element; 160]; 2]) {
    let le_bits_one: BitArray<_, Lsb0> = BitArray::new(commitment_one.to_le_bytes());
    let le_bits_two: BitArray<_, Lsb0> = BitArray::new(commitment_two.to_le_bytes());

    // Find the divergence point
    let mut divergence = 0;
    for i in 0..160 {
        if le_bits_one[i] != le_bits_two[i] {
            divergence = i;
        }
    }

    let mut hash_one = commitment_one;
    let mut hash_two = commitment_two;
    let mut path_one = [Element::ZERO; 160];
    let mut path_two = [Element::ZERO; 160];

    for i in 0..160 {
        if i == divergence {
            path_one[i] = hash_two;
            path_two[i] = hash_one;
            let next_hash = if !le_bits_one[i] {
                hash::hash_merge([hash_one, hash_two])
            } else {
                hash::hash_merge([hash_two, hash_one])
            };
            hash_one = next_hash;
            hash_two = next_hash;
        } else {
            let sibling = Element::new((i + 1000) as u64);
            path_one[i] = sibling;
            path_two[i] = sibling;
            hash_one = if !le_bits_one[i] {
                hash::hash_merge([hash_one, sibling])
            } else {
                hash::hash_merge([sibling, hash_one])
            };
            hash_two = if !le_bits_two[i] {
                hash::hash_merge([hash_two, sibling])
            } else {
                hash::hash_merge([sibling, hash_two])
            };
        }
    }

    (hash_one, [path_one, path_two])
}

async fn build_valid_inputs() -> Erc20TransferInput {
    let chain_id: u64 = 1;
    let bridge_address = Element::new(56789u64);
    let token_address: [u8; 20] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc,
        0xfe, 0x11, 0x22, 0x33, 0x44,
    ];
    let receiver: [u8; 20] = [
        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xab, 0xcd, 0xef, 0x01,
    ];

    let transfer_amount = 123456789;
    let nonce = 0;
    let max_priority_fee_per_gas = 2000000000;
    let max_fee_per_gas = 100000000000;
    let gas_limit = 21000;

    let original_transfer = Erc20transfer {
        chain_id,
        nonce,
        max_priority_fee_per_gas,
        max_fee_per_gas,
        gas_limit,
        token_address,
        receiver,
        amount: Element::new(transfer_amount),
    };

    let tx = TxEip1559 {
        chain_id,
        nonce,
        max_priority_fee_per_gas,
        max_fee_per_gas,
        gas_limit,
        to: Address::from(token_address).into(),
        value: U256::ZERO,
        input: build_erc20_transfer_calldata(receiver, transfer_amount),
        access_list: Default::default(),
    };

    let tx_hash = tx.signature_hash();

    let signer = PrivateKeySigner::from_str(
        "0x0000000000000000000000000000000000000000000000000000000000000001",
    )
    .expect("invalid private key");

    let pubkey = signer.public_key();
    let sender_pubkey_x: [u8; 32] = pubkey[..32].try_into().unwrap();
    let sender_pubkey_y: [u8; 32] = pubkey[32..].try_into().unwrap();

    let signature = signer.sign_hash(&tx_hash).await.expect("signing failed");
    let r: [u8; 32] = signature.r().to_be_bytes();
    let s: [u8; 32] = signature.s().to_be_bytes();

    let mut pubkey_concat = [0u8; 64];
    pubkey_concat[..32].copy_from_slice(&sender_pubkey_x);
    pubkey_concat[32..].copy_from_slice(&sender_pubkey_y);
    let sender_address: [u8; 20] = Keccak256::digest(pubkey_concat)[12..32].try_into().unwrap();

    let input_note_value_one = 70000000;
    let input_note_value_two = 80000000;

    let input_note_one = Note {
        kind: Element::ONE,
        address: sender_address,
        token: token_address,
        value: Element::new(input_note_value_one),
        psi: Element::new(42u64),
    };
    let input_note_two = Note {
        kind: Element::ONE,
        address: sender_address,
        token: token_address,
        value: Element::new(input_note_value_two),
        psi: Element::new(1000u64),
    };

    let commitment_one = hash_note(&input_note_one);
    let commitment_two = hash_note(&input_note_two);
    let (merkle_root, merkle_paths) = build_merkle_inputs(commitment_one, commitment_two);

    let new_nullifier_one = hash::hash_merge([commitment_one, input_note_one.psi]);
    let new_nullifier_two = hash::hash_merge([commitment_two, input_note_two.psi]);
    let new_nullifier_three = hash::hash_merge([
        Element::from_be_bytes(r),
        Element::new(chain_id),
        bridge_address,
    ]);

    let output_change = input_note_value_one + input_note_value_two - transfer_amount;
    let output_note_one = Note {
        kind: Element::ONE,
        address: sender_address,
        token: token_address,
        value: Element::new(output_change),
        psi: Element::new(99u64),
    };
    let output_note_two = Note {
        kind: Element::ONE,
        address: receiver,
        token: token_address,
        value: Element::new(transfer_amount),
        psi: Element::new(1234u64),
    };
    let new_commitments = [hash_note(&output_note_one), hash_note(&output_note_two)];

    Erc20TransferInput {
        chain_id,
        bridge_address,
        original_transfer,
        signature: SignatureStruct {
            r,
            s,
            sender_pubkey_x,
            sender_pubkey_y,
        },
        input_notes: [input_note_one, input_note_two],
        input_note_merkle_proofs: [
            Merklepath {
                path: merkle_paths[0],
            },
            Merklepath {
                path: merkle_paths[1],
            },
        ],
        input_note_merkle_root: merkle_root,
        new_nullifiers: [new_nullifier_one, new_nullifier_two, new_nullifier_three],
        output_notes: [output_note_one, output_note_two],
        new_commitments,
    }
}

#[tokio::test]
async fn test_erc20_transfer_accepts_inputs() {
    let input = build_valid_inputs().await;
    prove_and_verify(&input).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "UnsatisfiedConstrain")]
async fn test_erc20_transfer_rejects_bad_root() {
    let mut input = build_valid_inputs().await;
    input.input_note_merkle_root = input.input_note_merkle_root + Element::ONE;
    prove_and_verify(&input).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "UnsatisfiedConstrain")]
async fn test_erc20_transfer_rejects_bad_nullifier() {
    let mut input = build_valid_inputs().await;
    input.new_nullifiers[0] = input.new_nullifiers[0] + Element::ONE;
    prove_and_verify(&input).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "UnsatisfiedConstrain")]
async fn test_erc20_transfer_rejects_bad_commitment() {
    let mut input = build_valid_inputs().await;
    input.new_commitments[0] = input.new_commitments[0] + Element::ONE;
    prove_and_verify(&input).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "UnsatisfiedConstrain")]
async fn test_erc20_transfer_rejects_wrong_money_source() {
    let mut input = build_valid_inputs().await;
    // Change the first input note to belong to the receiver instead of the sender
    input.input_notes[0].address = input.original_transfer.receiver;
    // Rebuild merkle proofs for the modified notes
    let commitment_one = hash_note(&input.input_notes[0]);
    let commitment_two = hash_note(&input.input_notes[1]);
    let (root, paths) = build_merkle_inputs(commitment_one, commitment_two);
    input.input_note_merkle_root = root;
    input.input_note_merkle_proofs = [Merklepath { path: paths[0] }, Merklepath { path: paths[1] }];
    prove_and_verify(&input).await.unwrap();
}
