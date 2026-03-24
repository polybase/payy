use std::{hint::black_box, process::Command};

use benchy::{BenchmarkRun, benchmark};
use element::Element;
use hash::hash_merge;
use rand::thread_rng;
use smirk::{Batch, storage::Persistent};
use tempdir::TempDir;

fn make_batch(n: usize) -> Batch<160, ()> {
    let elements = core::iter::from_fn(|| Some(Element::secure_random(thread_rng()))).take(n);
    let mut batch = Batch::new();
    for element in elements {
        batch.insert(element, ()).unwrap();
    }
    batch
}

#[benchmark]
pub fn hash_merge_1_000_000(b: &mut BenchmarkRun) {
    let dir = TempDir::new("smirk-benchmark").unwrap();

    let batch = make_batch(1000);

    let mut persistent = Persistent::<160, ()>::new(dir.path()).unwrap();
    persistent.insert_batch(batch).unwrap();

    b.run(|| {
        let mut a = Element::new(4);
        let mut b = Element::new(5);
        for _ in 0..1_000_000 {
            (a, b) = (b, hash_merge([a, b]));
        }
    });

    b.metrics.insert("hash_count".into(), hash::hash_count());

    b.metrics
        .insert("hash_element_count".into(), hash::hash_element_count());
}

#[benchmark]
pub fn hash_merge_1_000_000_cached(b: &mut BenchmarkRun) {
    let dir = TempDir::new("smirk-benchmark").unwrap();

    let batch = make_batch(1000);

    let mut persistent = Persistent::<160, ()>::new(dir.path()).unwrap();
    persistent.insert_batch(batch).unwrap();

    let mut x = Element::new(4);
    let mut y = Element::new(5);
    for _ in 0..1_000_000 {
        (x, y) = (y, hash_merge([x, y]));
    }

    b.run(|| {
        let mut a = Element::new(4);
        let mut b = Element::new(5);
        for _ in 0..1_000_000 {
            (a, b) = (b, hash_merge([a, b]));
        }
    });

    b.metrics.insert("hash_count".into(), hash::hash_count());

    b.metrics
        .insert("hash_element_count".into(), hash::hash_element_count());
}

#[benchmark]
pub fn create_tree(b: &mut BenchmarkRun) {
    b.run(|| {
        let batch = make_batch(1000);

        let mut tree = smirk::Tree::<160, ()>::new();
        let _ = tree.insert_batch(batch, |_| {}, |_| {});

        black_box(tree);
    });

    b.metrics.insert("hash_count".into(), hash::hash_count());

    b.metrics
        .insert("hash_element_count".into(), hash::hash_element_count());
}

#[benchmark]
pub fn storage_load(b: &mut BenchmarkRun) {
    let dir = TempDir::new("smirk-benchmark").unwrap();

    let batch = make_batch(1000);

    let mut persistent = Persistent::<160, ()>::new(dir.path()).unwrap();
    persistent.insert_batch(batch).unwrap();

    // if we don't copy it to its own path, we get rocksdb errors
    let this_dir = TempDir::new("smirk-benchmark").unwrap();
    Command::new("cp")
        .arg("-r")
        .arg(dir.path())
        .arg(this_dir.path())
        .status()
        .unwrap();

    b.run(|| {
        let tree = Persistent::<160, ()>::load(this_dir.path()).unwrap();
        black_box(tree);
    });

    b.metrics.insert("hash_count".into(), hash::hash_count());

    b.metrics
        .insert("hash_element_count".into(), hash::hash_element_count());
}

benchy::main!(
    // hash_merge_1_000_000,
    // hash_merge_1_000_000_cached,
    create_tree,
    storage_load,
);
