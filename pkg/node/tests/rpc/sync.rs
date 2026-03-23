//! Tests for the sync RPC.
//! These can be very CPU intensive, since each spawn 4 nodes,
//! and Rust by default will run as many tests in parallel as you have cores.

// use node::PersistentMerkleTree;
// use prover::smirk_metadata::SmirkMetadata;
// use serial_test::serial;
// use element::Element;

// use super::{Server, ServerConfig};

// async fn setup() -> [Server; 4] {
//     let mut servers = ServerConfig::four_nodes(false).map(Server::new);

//     let peers = servers.iter().map(|s| s.to_peer()).collect::<Vec<_>>();

//     for server in &mut servers {
//         server.set_peers(&peers);
//         server.run(None);
//     }

//     futures::future::try_join_all(servers.iter().map(|server| server.wait_to_notice_sync()))
//         .await
//         .unwrap();

//     tokio::select! {
//         _ = servers[0].wait_for_height(2) => {}
//         _ = tokio::time::sleep(tokio::time::Duration::from_secs(20)) => {
//             panic!("Failed to wait for server a to reach height 2");
//         }
//     }

//     // Mint a note so that the root hash is not the empty tree root hash
//     let root_hash_before_mint = servers[0].height().await.unwrap().root_hash;
//     super::mint(
//         &servers[0],
//         Element::secure_random(&mut rand::thread_rng()),
//         Element::from(100u64),
//     )
//     .await
//     .unwrap();
//     let root_hash_after_mint = servers[0].height().await.unwrap().root_hash;
//     assert_ne!(
//         root_hash_before_mint, root_hash_after_mint,
//         "root hash before minting is the same as root hash after minting"
//     );

//     servers
// }

// #[tokio::test(flavor = "multi_thread")]
// #[serial]
// async fn block_db_missing() {
//     let [mut server_a, server_b, _server_c, _server_d] = setup().await;

//     server_a.stop();

//     server_a.reset_db();

//     server_a.run(None);
//     server_a.wait_to_notice_sync().await.expect("Failed to wait for server a");

//     let server_b_height = server_b.height().await.unwrap();
//     let server_a_height = server_a.height().await.unwrap();
//     assert!(
//         server_a_height.height >= server_b_height.height,
//         "server_a_height {} is less than server_b_height {}",
//         server_a_height.height,
//         server_b_height.height
//     );
//     assert_eq!(server_a_height.root_hash, server_b_height.root_hash);
// }

// #[tokio::test(flavor = "multi_thread")]
// #[serial]
// async fn smirk_missing() {
//     let [mut server_a, server_b, _server_c, _server_d] = setup().await;

//     server_a.stop();

//     server_a.reset_smirk();

//     server_a.run(None);
//     server_a.wait_to_notice_sync().await.expect("Failed to wait for server a");

//     let server_b_height = server_b.height().await.unwrap();
//     let server_a_height = server_a.height().await.unwrap();
//     assert!(
//         server_a_height.height >= server_b_height.height,
//         "server_a_height {} is less than server_b_height {}",
//         server_a_height.height,
//         server_b_height.height
//     );
//     assert_eq!(server_a_height.root_hash, server_b_height.root_hash);
// }

// #[tokio::test(flavor = "multi_thread")]
// #[serial]
// async fn db_and_smirk_missing() {
//     let [mut server_a, server_b, _server_c, _server_d] = setup().await;

//     server_a.stop();

//     server_a.reset_db();
//     server_a.reset_smirk();

//     server_a.run(None);
//     server_a.wait_to_notice_sync().await.expect("Failed to wait for server a");

//     let server_b_height = server_b.height().await.unwrap();
//     let server_a_height = server_a.height().await.unwrap();
//     assert!(
//         server_a_height.height >= server_b_height.height,
//         "server_a_height {} is less than server_b_height {}",
//         server_a_height.height,
//         server_b_height.height
//     );
//     assert_eq!(server_a_height.root_hash, server_b_height.root_hash);
// }

// #[tokio::test(flavor = "multi_thread")]
// #[serial]
// async fn missing_blocks() {
//     let [mut server_a, server_b, _server_c, _server_d] = setup().await;

//     let serve_a_height_before_stop = server_a.height().await.unwrap().height;
//     server_a.stop();
//     tokio::select! {
//         _ = server_b.wait_for_height(serve_a_height_before_stop + 2) => {}
//         _ = tokio::time::sleep(tokio::time::Duration::from_secs(20)) => {
//             panic!("Failed to wait for server b to reach height {}", serve_a_height_before_stop + 2);
//         }
//     }

//     server_a.run(None);
//     server_a.wait_to_notice_sync().await.expect("Failed to wait for server a");

//     let server_b_height = server_b.height().await.unwrap();
//     let server_a_height = server_a.height().await.unwrap();
//     assert!(
//         server_a_height.height >= server_b_height.height,
//         "server_a_height {} is less than server_b_height {}",
//         server_a_height.height,
//         server_b_height.height
//     );
//     assert_eq!(server_a_height.root_hash, server_b_height.root_hash);
// }

// #[tokio::test(flavor = "multi_thread")]
// #[serial]
// async fn corrupted_smirk() {
//     let [mut server_a, server_b, _server_c, _server_d] = setup().await;

//     server_a.stop();

//     let mut smirk =
//         PersistentMerkleTree::load(server_a.root_dir.path().join("smirk").join("latest")).unwrap();
//     smirk
//         .insert(
//             Element::secure_random(rand::thread_rng()),
//             SmirkMetadata::inserted_in(1),
//         )
//         .unwrap();
//     drop(smirk);

//     server_a.run(None);
//     server_a.wait_to_notice_sync().await.expect("Failed to wait for server a");

//     let server_b_height = server_b.height().await.unwrap();
//     let server_a_height = server_a.height().await.unwrap();
//     assert!(
//         server_a_height.height >= server_b_height.height,
//         "server_a_height {} is less than server_b_height {}",
//         server_a_height.height,
//         server_b_height.height
//     );
//     assert_eq!(server_a_height.root_hash, server_b_height.root_hash);
// }
