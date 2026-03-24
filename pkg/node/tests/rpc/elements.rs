use std::sync::Arc;

use barretenberg_cli::CliBackend;
use element::Element;
use hash::hash_merge;
use zk_primitives::{InputNote, Note, Utxo, bridged_polygon_usdc_note_kind};

use super::{Server, ServerConfig, mint, rollup_contract, usdc_contract};
use testutil::eth::EthNode;
use zk_circuits::Prove;

#[tokio::test(flavor = "multi_thread")]
async fn list_elements_unspent_default() {
    let eth_node = EthNode::default().run_and_deploy().await;
    let server_config = ServerConfig::single_node(false);
    let server = Server::setup_and_wait(server_config, Arc::clone(&eth_node)).await;
    let rollup = rollup_contract(server.rollup_contract_addr, &eth_node).await;
    let usdc = usdc_contract(&rollup, &eth_node).await;

    let alice_pk = Element::new(0xA11CE);
    let alice_address = hash_merge([alice_pk, Element::ZERO]);

    let (alice_note, eth_tx, node_tx) = mint(
        &rollup,
        &usdc,
        &server,
        alice_address,
        Element::from(10u64),
        Element::ZERO,
    )
    .await;
    eth_tx.await.unwrap();
    let tx = node_tx.await.unwrap();

    let commitment = alice_note.commitment();
    let list = server
        .list_elements(&[commitment], false)
        .await
        .expect("list_elements failed");
    assert_eq!(list.len(), 1, "expected exactly one element");
    let item = &list[0];
    assert_eq!(item.element, commitment);
    assert_eq!(item.height, tx.height.0);
    assert!(!item.spent, "expected unspent element to have spent=false");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore] // Requires Ethereum node - skip in CI
async fn list_elements_include_spent() {
    let eth_node = EthNode::default().run_and_deploy().await;
    let server =
        Server::setup_and_wait(ServerConfig::single_node(false), Arc::clone(&eth_node)).await;
    let rollup = rollup_contract(server.rollup_contract_addr, &eth_node).await;
    let usdc = usdc_contract(&rollup, &eth_node).await;

    let alice_pk = Element::new(0xA11CE);
    let alice_address = hash_merge([alice_pk, Element::ZERO]);
    let (alice_note, eth_tx, node_tx) = mint(
        &rollup,
        &usdc,
        &server,
        alice_address,
        Element::from(5u64),
        Element::ZERO,
    )
    .await;
    eth_tx.await.unwrap();
    let _tx = node_tx.await.unwrap();

    // Spend Alice's note to Bob so Alice's commitment is removed from the tree
    let bob_pk = Element::new(0xB0B);
    let bob_address = hash_merge([bob_pk, Element::ZERO]);
    let bob_note = Note::new_with_psi(
        bob_address,
        Element::from(5u64),
        Element::ZERO,
        bridged_polygon_usdc_note_kind(),
    );
    let input_note = InputNote::new(alice_note.clone(), alice_pk);
    let utxo = Utxo::new_send(
        [input_note, InputNote::padding_note()],
        [bob_note, Note::padding_note()],
    );
    let backend = CliBackend;
    let snark = utxo.prove(&backend).await.unwrap();
    let _resp = server.transaction(&snark).await.unwrap();

    let commitment = alice_note.commitment();

    // Default behavior should not include spent elements
    let list_default = server
        .list_elements(&[commitment], false)
        .await
        .expect("list_elements failed");
    assert!(
        list_default.is_empty(),
        "expected no elements without include_spent"
    );

    // With include_spent=true we should see it and it should be marked as spent
    let list_spent = server
        .list_elements(&[commitment], true)
        .await
        .expect("list_elements failed");
    assert_eq!(
        list_spent.len(),
        1,
        "expected one element with include_spent"
    );
    let item = &list_spent[0];
    assert_eq!(item.element, commitment);
    assert!(
        item.spent,
        "expected spent=true for previously spent element"
    );
}
