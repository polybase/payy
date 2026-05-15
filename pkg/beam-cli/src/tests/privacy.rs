use contextful::ErrorContextExt;
use json_store::JsonStoreError;
use tempfile::TempDir;

use crate::{
    error::Error,
    error_render::format_error_chain,
    privacy::{
        interface::PrivacyOperationRequest,
        interface::{BeamPrivacyClient, MockBeamPrivacyClient, PrivateAddressExport},
        state::{PrivacyState, PrivacyStateKey, load_privacy_state},
        validate_chain_public_key,
    },
    privacy_config::PrivacyProfile,
};

#[test]
fn evm_key_derives_deterministic_privacy_address() {
    let first = payy_evm_client::LocalPrivacySigner::from_evm_private_key([7u8; 32])
        .expect("derive privacy signer")
        .privacy_address();
    let second = payy_evm_client::LocalPrivacySigner::from_evm_private_key([7u8; 32])
        .expect("derive privacy signer")
        .privacy_address();
    let different = payy_evm_client::LocalPrivacySigner::from_evm_private_key([8u8; 32])
        .expect("derive different privacy signer")
        .privacy_address();

    assert_eq!(first, second);
    assert_ne!(first, different);
}

#[tokio::test]
async fn privacy_interface_is_mockable_without_payy_sdk_types() {
    let client = MockBeamPrivacyClient {
        address: PrivateAddressExport {
            evm_address: "0x1111111111111111111111111111111111111111".to_string(),
            private_address: format!("0x{}", "22".repeat(32)),
        },
        profile: PrivacyProfile::payy_default(),
    };

    client.validate().await.expect("validate mock privacy");
    assert_eq!(client.profile().standard, "payy-evm-privacy");
    let address = client.private_address();
    let balance = client
        .balance(contracts::Address::from_low_u64_be(1))
        .await
        .expect("read mock balance");
    let incoming = client
        .incoming(0, None, false)
        .await
        .expect("list mock incoming");
    let receipt = client
        .submit(PrivacyOperationRequest::Mint {
            amount_atomic: "1".to_string(),
            token: contracts::Address::from_low_u64_be(1),
        })
        .await
        .expect("submit mock operation");

    assert_eq!(address.private_address, format!("0x{}", "22".repeat(32)));
    assert_eq!(balance.spendable_atomic, "0");
    assert_eq!(incoming[0].status, "claimable");
    assert_eq!(receipt.operation, "mint");

    let _requests = [
        PrivacyOperationRequest::Burn {
            amount_atomic: "1".to_string(),
            recipient: contracts::Address::from_low_u64_be(2),
            token: contracts::Address::from_low_u64_be(1),
        },
        PrivacyOperationRequest::Send {
            amount_atomic: "1".to_string(),
            memo: None,
            recipient_private_address: format!("0x{}", "33".repeat(32)),
            token: contracts::Address::from_low_u64_be(1),
        },
        PrivacyOperationRequest::Claim {
            source: "incoming-id".to_string(),
        },
        PrivacyOperationRequest::EphemeralSend {
            amount_atomic: "1".to_string(),
            memo: None,
            token: contracts::Address::from_low_u64_be(1),
        },
    ];
}

#[test]
fn privacy_chain_public_key_rejects_unset_bridge_key() {
    let bridge = "0x3100000000000000000000000000000000000000"
        .parse::<contracts::Address>()
        .expect("parse bridge");
    let err = validate_chain_public_key("payy-testnet", bridge, &[0; 32], &[0; 32])
        .expect_err("reject unset key");

    assert!(matches!(
        err,
        Error::PrivacyFeatureUnsupported { chain, feature }
            if chain == "payy-testnet"
                && feature
                    == "bridge chain public key is not configured: 0x3100000000000000000000000000000000000000"
    ));
}

#[test]
fn privacy_chain_public_key_accepts_nonzero_bridge_key() {
    let bridge = "0x3100000000000000000000000000000000000000"
        .parse::<contracts::Address>()
        .expect("parse bridge");
    let mut x = [0u8; 32];
    x[31] = 1;

    validate_chain_public_key("payy-testnet", bridge, &x, &[0; 32]).expect("accept chain key");
}

#[test]
fn internal_error_format_includes_causes() {
    let err = Error::Internal(
        std::io::Error::other("root cause")
            .context("outer context")
            .into(),
    );

    let message = format_error_chain(&err);

    assert!(message.contains("[beam-cli] internal error"));
    assert!(message.contains("outer context: root cause"));
}

#[test]
fn privacy_state_key_rejects_stale_profile_metadata() {
    let key = state_key(1);
    let mut state = PrivacyState::default();
    state
        .entry_mut(&key)
        .expect("create matching privacy state entry");

    let stale = state_key(2);
    let err = state
        .entry(&stale)
        .expect_err("reject mismatched privacy standard version");

    assert!(matches!(err, Error::PrivacyStateNotFound { .. }));
}

#[test]
fn privacy_state_reset_removes_matching_entry() {
    let key = state_key(1);
    let mut state = PrivacyState::default();
    state
        .entry_mut(&key)
        .expect("create matching privacy state entry");

    state.entries.remove(&key.id());

    assert!(state.entry(&key).expect("read state").is_none());
}

#[tokio::test]
async fn privacy_state_fails_closed_on_invalid_json() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_path = temp_dir.path().join("privacy-state.json");
    std::fs::write(&file_path, "{ invalid json").expect("write invalid state");

    let err = match load_privacy_state(temp_dir.path()).await {
        Ok(_) => panic!("expected invalid privacy state to fail"),
        Err(err) => err,
    };

    match err {
        Error::Internal(internal) => match internal.recursive_downcast_ref::<JsonStoreError>() {
            Some(JsonStoreError::Deserialization { path, .. }) => assert_eq!(path, &file_path),
            other => panic!("unexpected json store error: {other:?}"),
        },
        other => panic!("unexpected error: {other:?}"),
    }
}

fn state_key(version: u32) -> PrivacyStateKey {
    PrivacyStateKey {
        bridge: "0x3100000000000000000000000000000000000000".to_string(),
        chain: "payy-testnet".to_string(),
        chain_id: 7298,
        privacy_address: format!("0x{}", "11".repeat(32)),
        standard: "payy-evm-privacy".to_string(),
        standard_version: version,
        wallet_address: "0x1111111111111111111111111111111111111111".to_string(),
    }
}
