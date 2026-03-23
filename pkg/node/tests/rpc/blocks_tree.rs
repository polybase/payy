use std::sync::Arc;

use barretenberg_cli::CliBackend;
use element::Element;
use hash::hash_merge;
use zk_circuits::Prove;
use zk_primitives::{InputNote, Note, Utxo, bridged_polygon_usdc_note_kind};

use super::{Server, ServerConfig, mint, rollup_contract, usdc_contract};
use testutil::eth::EthNode;

#[tokio::test(flavor = "multi_thread")]
async fn block_tree_include_block_outputs() {
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
        Element::from(10u64),
        Element::ZERO,
    )
    .await;
    eth_tx.await.unwrap();
    let tx = node_tx.await.unwrap();

    let response = server
        .block_tree(tx.height, None)
        .await
        .expect("fetch block elements");

    assert_eq!(response.height, tx.height);
    assert_eq!(response.root_hash, tx.root_hash);
    let tree_elements = response
        .elements
        .as_ref()
        .expect("full tree should be included when diff_from is not provided");
    assert!(
        tree_elements.contains(&alice_note.commitment()),
        "expected minted note commitment in block elements snapshot"
    );

    // Spend Alice's note to Bob so we exercise removals
    let bob_pk = Element::new(0xB0B);
    let bob_address = hash_merge([bob_pk, Element::ZERO]);
    let bob_note = Note::new_with_psi(
        bob_address,
        Element::from(10u64),
        Element::ZERO,
        bridged_polygon_usdc_note_kind(),
    );
    let input_note = InputNote::new(alice_note.clone(), alice_pk);
    let utxo = Utxo::new_send(
        [input_note, InputNote::padding_note()],
        [bob_note.clone(), Note::padding_note()],
    );
    let backend = CliBackend;
    let send_proof = utxo.prove(&backend).await.unwrap();
    let send_tx = server.transaction(&send_proof).await.unwrap();

    let post_send = server
        .block_tree(send_tx.height, Some(tx.height))
        .await
        .expect("fetch block elements after send");
    assert!(
        post_send.elements.is_none(),
        "diff responses should omit the full tree to avoid recomputation"
    );
    let diff = post_send
        .diff
        .expect("diff should be present when diff_from is requested");
    assert_eq!(diff.from_height, tx.height);
    assert_eq!(diff.additions, vec![bob_note.commitment()]);
    assert_eq!(diff.removals, vec![alice_note.commitment()]);
}
