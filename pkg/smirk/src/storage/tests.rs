use std::{collections::HashSet, path::PathBuf};

use tempdir::TempDir;
use test_strategy::proptest;

use crate::{batch, Batch};

use super::*;

fn setup_path() -> (TempDir, PathBuf) {
    let dir = TempDir::new("smirk_db_test").unwrap();
    let file = dir.path().join("db");

    (dir, file)
}

// the property tests here are pretty slow, so we limit the number of tests,
// unless `--features slow-storage-tests` is passed.
//
// for reference, running `insert_all_same_as_insert_lots_of_times`:
//  - on a laptop with a intel 1270p
//  - in release mode
//  - with `--features slow-storage-tests`
// takes ~2.5 minutes (though this number fluctuates a fair amount)

#[cfg(not(feature = "slow-storage-tests"))]
fn cases() -> u32 {
    10
}

#[cfg(feature = "slow-storage-tests")]
fn cases() -> u32 {
    proptest::test_runner::Config::default().cases / 2
}

#[test]
fn simple_storage_test() {
    let (_dir, path) = setup_path();
    let mut persistent = Persistent::<64, i32>::new(&path).unwrap();

    assert!(persistent.tree().is_empty());

    persistent.insert(Element::ONE, 1).unwrap();
    assert_eq!(persistent.tree().len(), 1);

    drop(persistent);

    // now load it again
    let persistent = Persistent::<64, i32>::load(&path).unwrap();
    assert!(persistent.tree().contains_element(Element::ONE));
    assert!(persistent.tree().get(Element::ONE) == Some(&1));
}

#[test]
fn persist_hashes_works() {
    let (_dir, path) = setup_path();
    let mut persistent = Persistent::<64, ()>::new(&path).unwrap();

    persistent.insert_batch(batch! { 2, 3 }).unwrap();

    persistent.persist_hashes().unwrap();

    drop(persistent);

    // now when we load the tree, we should see that it can load the tree, and the hash function in
    // `SimpleHashCache` should be used instead of plain `hash_merge`

    let persistent = Persistent::<64, ()>::load(&path).unwrap();
    assert!(persistent.tree().cache().metrics().hashes() > 0);
}

#[proptest(cases = cases())]
fn insert_batch_works(batch_1: Batch<64, i32>, mut batch_2: Batch<64, i32>) {
    let (_dir1, path) = setup_path();
    let mut persistent = Persistent::<64, i32>::new(&path).unwrap();

    for element in batch_1.elements() {
        batch_2.remove(element);
    }

    let batch_1_elements: HashSet<_> = batch_1.elements().collect();
    let batch_2_elements: HashSet<_> = batch_2.elements().collect();

    persistent.insert_batch(batch_1).unwrap();
    persistent.insert_batch(batch_2).unwrap();

    drop(persistent);

    let loaded = Persistent::<64, i32>::load(&path).unwrap();

    for element in batch_1_elements {
        assert!(loaded.tree().contains_element(element));
    }

    for element in batch_2_elements {
        assert!(loaded.tree().contains_element(element));
    }
}
