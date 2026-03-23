use super::SyncWorker;
use crate::{PersistentMerkleTree, block::Block, constants::MERKLE_TREE_DEPTH, types::BlockHeight};
use element::Element;
use prover::smirk_metadata::SmirkMetadata;
use zk_primitives::{UtxoProof, UtxoPublicInput};

fn e(n: u64) -> Element {
    Element::new(n)
}

fn make_block(height: u64, root: Element, inputs: [Element; 2], outputs: [Element; 2]) -> Block {
    let mut block = Block::default();
    block.content.header.height = BlockHeight(height);
    block.content.state.root_hash = root;
    block.content.state.txns = vec![UtxoProof {
        proof: Default::default(),
        public_inputs: UtxoPublicInput {
            input_commitments: inputs,
            output_commitments: outputs,
            messages: [Element::ZERO; 5],
        },
    }];
    block
}

fn compute_fast_snapshot_diffs(
    tree: &PersistentMerkleTree,
    block: &Block,
    elements: &[Element],
) -> (
    Vec<Element>,
    Vec<Element>,
    std::collections::HashMap<Element, bool>,
) {
    // Elements for the last block in the chunk (ignoring ZERO)
    let mut last_block_elements = std::collections::HashMap::<Element, bool>::new();
    for utxo in block.content.state.txns.iter() {
        for e in &utxo.public_inputs.input_commitments {
            if *e != Element::ZERO {
                last_block_elements.insert(*e, false);
            }
        }
        for e in &utxo.public_inputs.output_commitments {
            if *e != Element::ZERO {
                last_block_elements.insert(*e, true);
            }
        }
    }

    let elements_set: std::collections::HashSet<_> = elements.iter().copied().collect();
    let tree_elements_set: std::collections::HashSet<_> =
        tree.tree().elements().map(|(e, _)| *e).collect();

    let mut new_elements = elements_set
        .difference(&tree_elements_set)
        .copied()
        .collect::<Vec<_>>();
    let mut missing_elements = tree_elements_set
        .difference(&elements_set)
        .copied()
        .collect::<Vec<_>>();

    // Track last block elements that are still unaccounted for in the snapshot `elements`
    let mut block_elements_left_to_find = last_block_elements.clone();
    // Filter out last-block changes from the returned diffs (they are applied by the block)
    new_elements.retain(|e| {
        if last_block_elements.get(e) == Some(&true) {
            block_elements_left_to_find.remove(e);
            false
        } else {
            true
        }
    });

    missing_elements.retain(|e| {
        if last_block_elements.get(e) == Some(&false) {
            block_elements_left_to_find.remove(e);
            false
        } else {
            true
        }
    });

    (new_elements, missing_elements, block_elements_left_to_find)
}

#[test]
fn fast_snapshot_diff_computation() {
    let dir = tempdir::TempDir::new("fast_snapshot").unwrap();
    let tree_path = dir.path().join("db");
    let mut tree: PersistentMerkleTree = PersistentMerkleTree::new(&tree_path).unwrap();

    // Initial tree: {1,2,3}
    for v in [1u64, 2, 3] {
        tree.insert(Element::new(v), SmirkMetadata::inserted_in(0))
            .unwrap();
    }

    // Expected final state after to_height (elements vector): {2,4,5}
    let elements = vec![e(2), e(4), e(5)];

    // Last block removes 3, adds 5
    let block_root = tree.tree().root_hash_with(&[e(4), e(5)], &[e(1), e(3)]);
    let block = make_block(10, block_root, [e(3), Element::ZERO], [e(5), Element::ZERO]);

    let (new_elems, missing_elems, left_to_find) =
        compute_fast_snapshot_diffs(&tree, &block, &elements);

    let new_set: std::collections::HashSet<_> = new_elems.into_iter().collect();
    let missing_set: std::collections::HashSet<_> = missing_elems.into_iter().collect();
    assert_eq!(new_set, [e(4)].into_iter().collect());
    assert_eq!(missing_set, [e(1)].into_iter().collect());
    assert!(left_to_find.is_empty());
}

#[test]
fn fast_snapshot_root_mismatch_does_not_mutate_tree() {
    let dir = tempdir::TempDir::new("fast_snapshot_mismatch").unwrap();
    let mut tree: PersistentMerkleTree = PersistentMerkleTree::new(dir.path().join("db")).unwrap();

    // Initial tree: {1,2}
    for v in [1u64, 2] {
        tree.insert(Element::new(v), SmirkMetadata::inserted_in(0))
            .unwrap();
    }

    // Elements claim state {2,3}
    let elements = vec![e(2), e(3)];

    // Incorrect root (use empty tree root to ensure mismatch)
    let wrong_root = smirk::empty_tree_hash(MERKLE_TREE_DEPTH);
    let block = make_block(5, wrong_root, [Element::ZERO; 2], [Element::ZERO; 2]);

    // Capture before state
    let before: std::collections::HashSet<_> = tree.tree().elements().map(|(e, _)| *e).collect();

    SyncWorker::apply_fast_snapshot_chunk(&mut tree, &block, &elements).unwrap();

    // Tree unchanged
    let after: std::collections::HashSet<_> = tree.tree().elements().map(|(e, _)| *e).collect();
    assert_eq!(before, after);
}

#[test]
fn fast_snapshot_missing_last_block_elements_diff_computation() {
    let dir = tempdir::TempDir::new("fast_snapshot_missing_last").unwrap();
    let mut tree: PersistentMerkleTree = PersistentMerkleTree::new(dir.path().join("db")).unwrap();

    // Initial tree: {1,3}
    for v in [1u64, 3] {
        tree.insert(Element::new(v), SmirkMetadata::inserted_in(0))
            .unwrap();
    }

    // Final intended state after block: remove 3, add 5, plus 2
    let elements_full = vec![e(1), e(2), e(5)];
    let expected_root = tree.tree().root_hash_with(&[e(2), e(5)], &[e(3)]);

    // Last block removes 3, adds 5
    let block = make_block(
        7,
        expected_root,
        [e(3), Element::ZERO],
        [e(5), Element::ZERO],
    );

    // Provide an elements vector missing the last block's add (5)
    let elements_missing = vec![e(1), e(2)];

    let (_new_full, _missing_full, left_full) =
        compute_fast_snapshot_diffs(&tree, &block, &elements_full);
    assert!(left_full.is_empty());

    let (_new_missing, _missing_missing, left_missing) =
        compute_fast_snapshot_diffs(&tree, &block, &elements_missing);
    assert!(left_missing.contains_key(&e(5))); // last block add is missing from snapshot elements
}
