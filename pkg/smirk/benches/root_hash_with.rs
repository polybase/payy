use std::hint::black_box;

use benchy::{BenchmarkRun, benchmark};
use element::Element;
use rand::thread_rng;
use smirk::{Batch, Tree};

const TREE_DEPTH: usize = 160;
const TREE_SIZE: usize = 50_000;

fn populate_tree() -> Tree<TREE_DEPTH, ()> {
    let mut tree = Tree::<TREE_DEPTH, ()>::new();
    let mut batch = Batch::with_capacity(TREE_SIZE);
    let mut rng = thread_rng();

    while batch.insert_elements().count() < TREE_SIZE {
        let candidate = Element::secure_random(&mut rng);
        if batch.insert(candidate, ()).is_ok() {
            continue;
        }
    }

    tree.insert_batch(batch, |_| {}, |_| {}).unwrap();

    tree
}

fn random_insert_elements(tree: &Tree<TREE_DEPTH, ()>) -> [Element; 2] {
    let mut inserts = Vec::with_capacity(2);
    let mut rng = thread_rng();

    while inserts.len() < 2 {
        let candidate = Element::secure_random(&mut rng);
        if inserts.contains(&candidate) {
            continue;
        }

        if tree.contains_element(&candidate) {
            continue;
        }

        inserts.push(candidate);
    }

    [inserts[0], inserts[1]]
}

#[benchmark]
pub fn root_hash_with_two_inserts_one_remove(b: &mut BenchmarkRun) {
    let tree = populate_tree();
    let insert_elements = random_insert_elements(&tree);
    let remove_element = tree
        .elements()
        .next()
        .map(|(element, _)| *element)
        .expect("tree populated");
    let remove_elements = [remove_element];

    b.run(|| {
        black_box(tree.root_hash_with(&insert_elements, &remove_elements));
    });
}

benchy::main!(root_hash_with_two_inserts_one_remove);
