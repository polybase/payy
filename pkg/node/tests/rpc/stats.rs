use std::sync::Arc;

use element::Element;
use hash::hash_merge;

use super::{Server, ServerConfig, mint, rollup_contract, usdc_contract};
use testutil::eth::EthNode;

#[tokio::test(flavor = "multi_thread")]
async fn test_stats_today_counter() {
    let eth_node = EthNode::default().run_and_deploy().await;
    let server_config = ServerConfig::single_node(false);
    let server = Server::setup_and_wait(server_config, Arc::clone(&eth_node)).await;

    // Wait until stats are ready
    let mut stats = loop {
        match server.stats().await {
            Ok(s) => break s,
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    };

    // Initial stats
    assert_eq!(stats.today_txns, 0);

    let rollup = rollup_contract(server.rollup_contract_addr, &eth_node).await;
    let usdc = usdc_contract(&rollup, &eth_node).await;

    let alice_pk = Element::new(0xA11CE);
    let alice_address = hash_merge([alice_pk, Element::ZERO]);

    // Mint a transaction
    let (_alice_note, eth_tx, node_tx) = mint(
        &rollup,
        &usdc,
        &server,
        alice_address,
        Element::from(10u64),
        Element::ZERO,
    )
    .await;
    eth_tx.await.unwrap();
    node_tx.await.unwrap();

    // Query stats again
    stats = server.stats().await.expect("stats failed");
    assert_eq!(stats.today_txns, 1, "Expected 1 transaction today");
}
