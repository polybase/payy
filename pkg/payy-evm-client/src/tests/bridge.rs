use std::sync::Arc;

use payy_evm_client_interface::PayyNetworkPreset;

use super::MockReadClient;
use crate::BaseClient;

#[tokio::test]
async fn base_bridge_reads_use_adapter_shape() {
    let config = PayyNetworkPreset::Dev.config();
    let root = [7u8; 32].to_vec();
    let read_client = Arc::new(MockReadClient {
        chain_id: config.chain_id,
        block_number: 11,
        read_response: root,
    });
    let client = BaseClient::builder(config, read_client).build();

    assert_eq!(client.bridge().get_root().await.unwrap(), [7u8; 32]);
    assert_eq!(client.transactions(), crate::TransactionsClient);
}

#[tokio::test]
async fn base_bridge_merkle_path_returns_paired_root() {
    let config = PayyNetworkPreset::Dev.config();
    let root = [7u8; 32];
    let siblings = vec![[8u8; 32], [9u8; 32]];
    let read_client = Arc::new(MockReadClient {
        chain_id: config.chain_id,
        block_number: 11,
        read_response: encode_merkle_path(root, &siblings),
    });
    let client = BaseClient::builder(config, read_client).build();

    let path = client.bridge().get_merkle_path([1u8; 32]).await.unwrap();

    assert_eq!(path.root, root);
    assert_eq!(path.siblings, siblings);
}

#[tokio::test]
async fn base_bridge_element_exists_requires_canonical_bool() {
    let config = PayyNetworkPreset::Dev.config();
    let false_client = BaseClient::builder(
        config,
        Arc::new(MockReadClient {
            chain_id: config.chain_id,
            block_number: 11,
            read_response: word_from_u64(0).to_vec(),
        }),
    )
    .build();
    let true_client = BaseClient::builder(
        config,
        Arc::new(MockReadClient {
            chain_id: config.chain_id,
            block_number: 11,
            read_response: word_from_u64(1).to_vec(),
        }),
    )
    .build();
    let malformed_client = BaseClient::builder(
        config,
        Arc::new(MockReadClient {
            chain_id: config.chain_id,
            block_number: 11,
            read_response: word_from_u64(2).to_vec(),
        }),
    )
    .build();

    assert!(!false_client.bridge().element_exists([0; 32]).await.unwrap());
    assert!(true_client.bridge().element_exists([0; 32]).await.unwrap());
    assert!(
        malformed_client
            .bridge()
            .element_exists([0; 32])
            .await
            .is_err()
    );
}

fn encode_merkle_path(root: [u8; 32], siblings: &[[u8; 32]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&root);
    bytes.extend_from_slice(&word_from_u64(64));
    bytes.extend_from_slice(&word_from_u64(siblings.len() as u64));
    for sibling in siblings {
        bytes.extend_from_slice(sibling);
    }
    bytes
}

fn word_from_u64(value: u64) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&value.to_be_bytes());
    word
}
