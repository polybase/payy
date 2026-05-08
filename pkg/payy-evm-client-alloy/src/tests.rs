use alloy::primitives::{Address as AlloyAddress, TxKind, U256};
use payy_evm_client_interface::{
    Error, PayyEvmTransactionRequest, PreparedOperationResult, PreparedPrivacyCall, PrivacyAddress,
    PrivacyOperationKind, PrivacyStatePreview, ValidationErrorKind,
};

use crate::adapter::{receipt_confirmed, receipt_timeout_error};
use crate::{AlloyTransactionOptions, to_alloy_transaction, to_alloy_transaction_with_options};

const CHAIN_ID: u64 = 7298;
const FROM: [u8; 20] = [1u8; 20];
const TO: [u8; 20] = [2u8; 20];
const DATA: [u8; 3] = [0xab, 0xcd, 0xef];
const GAS_LIMIT: u64 = 123_456;
const VALUE: u128 = 77;

#[test]
fn converts_prepared_privacy_call_to_alloy_transaction() {
    let tx = to_alloy_transaction(&prepared_call()).expect("convert prepared call");

    assert_eq!(tx.chain_id, Some(CHAIN_ID));
    assert_eq!(tx.from, Some(AlloyAddress::from(FROM)));
    assert_eq!(tx.to, Some(TxKind::Call(AlloyAddress::from(TO))));
    assert_eq!(tx.gas, Some(GAS_LIMIT));
    assert_eq!(tx.value, Some(U256::from(VALUE)));
    assert_eq!(
        tx.input.input.expect("transaction input").as_ref(),
        DATA.as_slice()
    );
}

#[test]
fn converts_prepared_operation_result_to_alloy_transaction() {
    let result = PreparedOperationResult {
        prepared_call: prepared_call(),
        payload: (),
    };

    let tx = to_alloy_transaction(&result).expect("convert prepared result");

    assert_eq!(tx.chain_id, Some(CHAIN_ID));
    assert_eq!(tx.from, Some(AlloyAddress::from(FROM)));
}

#[test]
fn rejects_chain_id_mismatch() {
    let err = to_alloy_transaction_with_options(
        &prepared_call(),
        AlloyTransactionOptions {
            chain_id: Some(1),
            from: None,
        },
    )
    .expect_err("reject chain mismatch");

    assert!(matches!(
        err,
        Error::ChainIdMismatch {
            expected: CHAIN_ID,
            actual: 1,
        }
    ));
}

#[test]
fn rejects_sender_mismatch() {
    let err = to_alloy_transaction_with_options(
        &prepared_call(),
        AlloyTransactionOptions {
            chain_id: None,
            from: Some([9u8; 20]),
        },
    )
    .expect_err("reject sender mismatch");

    assert!(matches!(
        err,
        Error::Validation {
            kind: ValidationErrorKind::EvmAccountMismatch,
        }
    ));
}

#[test]
fn receipt_confirmation_helper_honors_requested_depth() {
    assert!(receipt_confirmed(10, 10, None));
    assert!(receipt_confirmed(10, 10, Some(1)));
    assert!(receipt_confirmed(10, 12, Some(3)));
    assert!(!receipt_confirmed(10, 11, Some(3)));
    assert!(receipt_confirmed(10, 10, Some(0)));
}

#[test]
fn receipt_timeout_uses_dedicated_error() {
    let hash = [13u8; 32];
    let err = receipt_timeout_error(hash, 123);

    assert!(matches!(
        err,
        Error::ReceiptTimeout {
            hash: actual_hash,
            timeout_ms: 123,
        } if actual_hash == hash
    ));
}

fn prepared_call() -> PreparedPrivacyCall {
    PreparedPrivacyCall {
        operation: PrivacyOperationKind::Mint,
        chain_id: CHAIN_ID,
        bridge_request: PayyEvmTransactionRequest {
            from: Some(FROM),
            to: TO,
            data: DATA.to_vec(),
            value: VALUE,
            gas_limit: Some(GAS_LIMIT),
        },
        verification_key_hash: [3u8; 32],
        proof: vec![4u8; 8],
        public_inputs: vec![[5u8; 32]],
        tx_commitment: [6u8; 32],
        state_preview: PrivacyStatePreview {
            privacy_account: PrivacyAddress::new([7u8; 32]),
            token: [8u8; 20],
            recent_root: [9u8; 32],
            input_commitments: vec![[10u8; 32]],
            input_nullifiers: vec![[11u8; 32]],
            output_commitments: vec![[12u8; 32]],
        },
    }
}
