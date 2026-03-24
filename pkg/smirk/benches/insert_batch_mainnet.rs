use std::hint::black_box;

use benchy::{BenchmarkRun, benchmark};
use smirk::{Batch, Tree, storage::Persistent};
use tempdir::TempDir;

const TREE_DEPTH: usize = 161;

mod data_download {
    use element::Element;
    use node_client_http::NodeClientHttp;
    use primitives::block_height::BlockHeight;
    use reqwest::Url;
    use std::sync::OnceLock;

    const TARGET_HEIGHT: u64 = 13_226_573; // non-empty diff as of 2026-02-04
    const BASE_URL: &str = "https://validators.mainnet.payy.network/v0";

    pub fn test_data() -> &'static (Vec<Element>, Vec<Element>, Vec<Element>) {
        TEST_DATA.get_or_init(download_data)
    }

    static TEST_DATA: OnceLock<(Vec<Element>, Vec<Element>, Vec<Element>)> = OnceLock::new();

    fn download_data() -> (Vec<Element>, Vec<Element>, Vec<Element>) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = NodeClientHttp::new(Url::parse(BASE_URL).unwrap());
            let snapshot = client
                .block_tree(BlockHeight(TARGET_HEIGHT - 1))
                .await
                .unwrap();
            let diff = client
                .block_tree_diff(BlockHeight(TARGET_HEIGHT), BlockHeight(TARGET_HEIGHT - 1))
                .await
                .unwrap();
            (snapshot.elements, diff.diff.additions, diff.diff.removals)
        })
    }
}

#[benchmark]
pub fn build_tree_from_snapshot(b: &mut BenchmarkRun) {
    let (elements, _, _) = data_download::test_data();

    b.run(|| {
        let mut tree = Tree::<TREE_DEPTH, ()>::new();
        let batch = Batch::<TREE_DEPTH, ()>::from_entries(
            elements.iter().copied().map(|element| (element, ())),
            [],
        )
        .unwrap();
        tree.insert_batch(batch, |_| {}, |_| {}).unwrap();
        black_box(tree);
    });
}

#[benchmark]
pub fn insert_batch_single_block(b: &mut BenchmarkRun) {
    let (elements, additions, removals) = data_download::test_data();

    let mut tree = Tree::<TREE_DEPTH, ()>::new();
    let batch = Batch::<TREE_DEPTH, ()>::from_entries(
        elements.iter().copied().map(|element| (element, ())),
        [],
    )
    .unwrap();
    tree.insert_batch(batch, |_| {}, |_| {}).unwrap();

    b.run(|| {
        let batch = Batch::<TREE_DEPTH, ()>::from_entries(
            black_box(additions)
                .iter()
                .copied()
                .map(|element| (element, ())),
            black_box(removals).iter().copied(),
        )
        .unwrap();

        tree.insert_batch(batch, |_| {}, |_| {}).unwrap();
        black_box(tree);
    });
}

#[benchmark]
pub fn load_tree_from_rocksdb(b: &mut BenchmarkRun) {
    let (elements, _, _) = data_download::test_data();
    let dir = TempDir::new("smirk-benchmark").unwrap();

    let batch = Batch::<TREE_DEPTH, ()>::from_entries(
        elements.iter().copied().map(|element| (element, ())),
        [],
    )
    .unwrap();
    let mut persistent = Persistent::<TREE_DEPTH, ()>::new(dir.path()).unwrap();
    persistent.insert_batch(batch).unwrap();
    drop(persistent);

    b.run(|| {
        let loaded = Persistent::<TREE_DEPTH, ()>::load(dir.path()).unwrap();
        black_box(loaded);
    });
}

benchy::main!(
    build_tree_from_snapshot,
    insert_batch_single_block,
    load_tree_from_rocksdb
);
