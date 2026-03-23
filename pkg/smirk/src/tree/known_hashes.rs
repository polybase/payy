use crate::{Tree, hash_cache::KnownHash};

use super::tree_repr::Node;

impl<const DEPTH: usize, V, C> Tree<DEPTH, V, C> {
    pub(crate) fn known_hashes(&self) -> Vec<KnownHash> {
        self.tree.known_hashes(DEPTH * self.len())
    }
}

impl Node {
    pub(crate) fn known_hashes(&self, cap: usize) -> Vec<KnownHash> {
        let mut hashes = Vec::with_capacity(cap);
        self.known_hashes_inner(&mut hashes);
        hashes
    }

    fn known_hashes_inner(&self, hashes: &mut Vec<KnownHash>) {
        match self {
            Node::Leaf(_) | Node::Empty { .. } => {}
            Node::Parent {
                left,
                right,
                hash,
                hash_dirty,
            } => {
                assert!(!hash_dirty, "hash should never be dirty in normal use");

                let known_hash = KnownHash {
                    left: left.hash(),
                    right: right.hash(),
                    result: *hash,
                };

                hashes.push(known_hash);
                left.known_hashes_inner(hashes);
                right.known_hashes_inner(hashes);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use element::Element;
    use hash::hash_merge;

    use crate::{hash::empty_tree_hash, smirk};

    use super::*;

    #[test]
    fn can_get_known_hashes() {
        let tree: Tree<3, ()> = smirk! {  2, 3 };
        let hashes = Vec::from_iter(tree.known_hashes());

        // 2-3 are in 1 subtree
        // the other subtree is `hash(2, 3)` and `empty(2)`

        let two_three = hash_merge([2, 3].map(Element::new));
        let root = hash_merge([empty_tree_hash(2), two_three]);

        let expected = vec![
            KnownHash {
                left: empty_tree_hash(2),
                right: two_three,
                result: root,
            },
            KnownHash {
                left: Element::new(2),
                right: Element::new(3),
                result: two_three,
            },
        ];

        assert_eq!(hashes, expected);
    }
}
