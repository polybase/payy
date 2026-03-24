use std::sync::Arc;

use element::Element;
use testutil::eth::EthNode;

use crate::rpc::{ServerConfig, mint, rollup_contract};

use super::usdc_contract;

#[tokio::test(flavor = "multi_thread")]
async fn smirk_elements_export() {
    let eth_node = EthNode::default().run_and_deploy().await;
    let server =
        super::Server::setup_and_wait(ServerConfig::single_node(false), Arc::clone(&eth_node))
            .await;
    let rollup = rollup_contract(server.rollup_contract_addr, &eth_node).await;
    let usdc = usdc_contract(&rollup, &eth_node).await;

    let (note1, eth_mint_tx1, tx1) = mint(
        &rollup,
        &usdc,
        &server,
        Element::new(1),
        Element::from(100u64),
        Element::ZERO,
    )
    .await;
    eth_mint_tx1.await.unwrap();
    let tx_resp1 = tx1.await.unwrap();

    let smirk_elements = server.get_all_smirk_elements().await.unwrap();

    assert_eq!(smirk_elements.len(), 1);

    assert!(
        !smirk_elements.is_empty(),
        "Smirk elements list should not be empty after minting."
    );

    let minted_element_info = smirk_elements
        .iter()
        .find(|info| info.element == note1.commitment());
    assert!(
        minted_element_info.is_some(),
        "Minted note commitment should be in the smirk elements list."
    );

    let info = minted_element_info.unwrap();
    assert_eq!(info.element, note1.commitment());
    assert_eq!(
        info.inserted_at_height, tx_resp1.height.0,
        "Inserted_at_height should match the block height of the mint transaction."
    );
}
