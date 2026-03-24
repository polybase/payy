use constants::MERKLE_TREE_DEPTH;
use testutil::eth::EthNode;

use crate::rpc::ServerConfig;

use super::Server;

#[tokio::test(flavor = "multi_thread")]
async fn empty() {
    let eth_node = EthNode::default().run_and_deploy().await;
    let server = Server::setup_and_wait(ServerConfig::single_node(false), eth_node).await;
    let resp = server.height().await.unwrap();
    assert_eq!(resp.root_hash, smirk::empty_tree_hash(MERKLE_TREE_DEPTH));
}
