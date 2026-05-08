use std::sync::Arc;

use async_trait::async_trait;
use payy_evm_client_interface::{
    Address, OwnedNoteState, PayyEvmSubmitter, PayyEvmTransactionRequest, PayyNetworkPreset,
    PrivacyAddress, Result, TxHash,
};

use super::{MockReadClient, parse_b256};
use crate::{BaseClient, LocalPrivacySigner, PrivacyClient};

struct MockSubmitter {
    chain_id: u64,
    address: Address,
}

#[async_trait]
impl PayyEvmSubmitter for MockSubmitter {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.chain_id)
    }

    async fn get_address(&self) -> Result<Option<Address>> {
        Ok(Some(self.address))
    }

    async fn send_transaction(&self, _request: PayyEvmTransactionRequest) -> Result<TxHash> {
        Ok([0x33; 32])
    }
}

#[test]
fn base_client_exposes_explicit_local_private_key_builders() {
    let config = PayyNetworkPreset::Dev.config();
    let read_client = Arc::new(MockReadClient {
        chain_id: config.chain_id,
        block_number: 11,
        read_response: Vec::new(),
    });
    let client = BaseClient::builder(config, read_client).build();
    let evm_private_key = [0x11u8; 32];
    let evm_client = client
        .clone()
        .with_evm_private_key(evm_private_key)
        .unwrap();
    let evm_address = default_privacy_address(&evm_client);

    assert_eq!(
        default_privacy_address(
            &client
                .clone()
                .with_secp256k1_private_key(evm_private_key)
                .unwrap(),
        ),
        evm_address
    );

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/payy-evm-client/v3.json")).unwrap();
    let grumpkin_private_key = parse_b256(fixture["grumpkin_private_key"].as_str().unwrap());
    let expected_address = LocalPrivacySigner::from_grumpkin_private_key(grumpkin_private_key)
        .unwrap()
        .privacy_address();

    assert_eq!(
        default_privacy_address(
            &client
                .with_grumpkin_private_key(grumpkin_private_key)
                .unwrap()
        ),
        expected_address
    );
}

#[test]
fn derived_clients_share_checkpoint_cache() {
    let config = PayyNetworkPreset::Dev.config();
    let read_client = Arc::new(MockReadClient {
        chain_id: config.chain_id,
        block_number: 11,
        read_response: Vec::new(),
    });
    let submitter = Arc::new(MockSubmitter {
        chain_id: config.chain_id,
        address: [0xaa; 20],
    });
    let signer = Arc::new(LocalPrivacySigner::from_grumpkin_private_key([0x11; 32]).unwrap());
    let privacy_client = BaseClient::builder(config, read_client.clone())
        .build()
        .privacy_signer(signer.clone());
    let account = privacy_client
        .privacy()
        .default_account()
        .unwrap()
        .unwrap()
        .privacy_address();
    let token = [0x01; 20];
    let initial_checkpoint = checkpoint(account, token, 9);

    privacy_client
        .privacy()
        .set_checkpoint(initial_checkpoint.clone())
        .unwrap();
    let derived = privacy_client.clone().evm_signer(submitter.clone());

    assert!(Arc::ptr_eq(
        &privacy_client.inner.checkpoints,
        &derived.inner.checkpoints
    ));
    assert_eq!(
        cached_checkpoint(&derived, account, token),
        Some(initial_checkpoint)
    );

    let later = checkpoint(account, token, 10);
    derived.privacy().set_checkpoint(later.clone()).unwrap();

    assert_eq!(
        cached_checkpoint(&privacy_client, account, token),
        Some(later)
    );

    let base = BaseClient::builder(config, read_client).build();
    let evm_first = base.clone().evm_signer(submitter);
    let privacy_after = evm_first.privacy_signer(signer);
    let after_checkpoint = checkpoint(account, [0x02; 20], 11);
    privacy_after
        .privacy()
        .set_checkpoint(after_checkpoint.clone())
        .unwrap();
    let sibling_privacy = base.with_grumpkin_private_key([0x11; 32]).unwrap();

    assert_eq!(
        cached_checkpoint(&sibling_privacy, account, [0x02; 20]),
        Some(after_checkpoint)
    );
}

fn default_privacy_address(client: &PrivacyClient) -> payy_evm_client_interface::PrivacyAddress {
    client
        .privacy()
        .default_account()
        .unwrap()
        .unwrap()
        .privacy_address()
}

fn checkpoint(
    privacy_account: PrivacyAddress,
    token: Address,
    checked_block: u64,
) -> OwnedNoteState {
    OwnedNoteState {
        privacy_account,
        token,
        owned_note: None,
        checked_block,
    }
}

fn cached_checkpoint(
    client: &PrivacyClient,
    privacy_account: PrivacyAddress,
    token: Address,
) -> Option<OwnedNoteState> {
    client
        .inner
        .checkpoints
        .lock()
        .unwrap()
        .get(&(privacy_account, token))
        .cloned()
}
