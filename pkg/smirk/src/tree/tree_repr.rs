use bitvec::{prelude::Msb0, slice::BitSlice, vec::BitVec};

use crate::{hash::empty_tree_hash, hash_cache::HashCache, hash_merge, Collision, Element};

use super::StructName;

/// A tree-like representation of a sparse tree, for easier computation of merkle paths and hashes
#[derive(Debug, Clone)]
pub(crate) enum Node {
    /// A single leaf at the max depth of the tree
    Leaf(Element),

    /// A tree of depth `depth` containing only null elements
    ///
    /// Since these trees are well-known, all hashes can be computed ahead of time and refered to
    /// by lookup table
    Empty { depth: usize },

    /// A parent of two nodes with a cached hash
    Parent {
        left: Box<Self>,
        right: Box<Self>,
        hash: Element,
        /// if true, the children have changed without recalculating the hash
        hash_dirty: bool,
    },
}

impl Node {
    pub fn hash_with<const DEPTH: usize>(
        &self,
        extra_elements: &[Element],
        path: &BitSlice,
    ) -> Element {
        fn make_paths(path: &BitSlice) -> (BitVec, BitVec) {
            let mut left_path = path.to_bitvec();
            left_path.push(false);
            let mut right_path = path.to_bitvec();
            right_path.push(true);

            (left_path, right_path)
        }

        match self {
            Self::Leaf(element) => *element,
            Self::Parent { left, right, .. } => {
                let (left_path, right_path) = make_paths(path);

                let left_hash = left.hash_with::<DEPTH>(extra_elements, &left_path);
                let right_hash = right.hash_with::<DEPTH>(extra_elements, &right_path);

                hash_merge([left_hash, right_hash])
            }
            Self::Empty { depth: 1 } => {
                // we need to check whether there should be an element here
                extra_elements
                    .iter()
                    .copied()
                    .find(|e| e.lsb(DEPTH - 1).starts_with(path))
                    .unwrap_or(empty_tree_hash(1))
            }
            Self::Empty { depth } => {
                // are there any elements that need to be "inserted" into this subtree?
                let subtree_has_extra_elements = extra_elements
                    .iter()
                    .any(|e| e.lsb(DEPTH - 1).starts_with(path));

                if subtree_has_extra_elements {
                    // if we need to, split it into two subtrees and reuse the logic from the
                    // Parent case

                    let (left_path, right_path) = make_paths(path);

                    let child = Self::Empty { depth: depth - 1 };

                    let left_hash = child.hash_with::<DEPTH>(extra_elements, &left_path);
                    let right_hash = child.hash_with::<DEPTH>(extra_elements, &right_path);

                    hash_merge([left_hash, right_hash])
                } else {
                    // otherwise, we can just use the standard hash for this empty tree
                    empty_tree_hash(*depth)
                }
            }
        }
    }

    pub fn hash(&self) -> Element {
        match self {
            Self::Leaf(hash) | Self::Parent { hash, .. } => *hash,
            Self::Empty { depth } => empty_tree_hash(*depth),
        }
    }

    /// Insert an element and return whether the value changed
    ///
    /// This does not update hashes, instead it marks nodes as "dirty" meaning the hash is
    /// potentially out of date
    pub(crate) fn insert_without_hashing<const N: usize>(
        &mut self,
        element: Element,
        bits: &BitSlice<u8, Msb0>,
    ) -> Result<bool, Collision> {
        match self {
            Self::Leaf(e) if *e == element => Ok(false),
            Self::Leaf(e) if e.lsb(N - 1) == element.lsb(N - 1) => Err(Collision {
                in_tree: *e,
                inserted: element,
                depth: N,
                struct_name: StructName::Tree,
            }),
            Self::Leaf(_) => unreachable!(),
            // Self::Leaf(e) => {
            //
            //     dbg!(&e, &element, e.lsb(N - 1), element.lsb(N - 1));
            //     *e = element;
            //     Ok(true)
            // }
            Self::Parent {
                left,
                right,
                hash_dirty,
                ..
            } => {
                let (head, tail) = bits.split_first().unwrap();
                let result = match *head {
                    false => left.insert_without_hashing::<N>(element, tail),
                    true => right.insert_without_hashing::<N>(element, tail),
                };

                if matches!(result, Ok(true)) {
                    *hash_dirty = true;
                }

                result
            }
            Self::Empty { depth: 1 } => {
                *self = Self::Leaf(element);
                Ok(true)
            }

            Self::Empty { depth } => {
                // split an empty tree into two empty subtrees
                *self = Self::Parent {
                    left: Box::new(Self::Empty { depth: *depth - 1 }),
                    right: Box::new(Self::Empty { depth: *depth - 1 }),
                    // This value is arbitrary, since it is immediately overwritten (since the node
                    // has `hash_dirty: true`)
                    hash: Element::NULL_HASH,
                    hash_dirty: false,
                };

                // now try again
                self.insert_without_hashing::<N>(element, bits)
            }
        }
    }

    pub fn recalculate_hashes<C: HashCache>(&mut self, cache: &C) {
        let Self::Parent {
            left,
            right,
            hash,
            hash_dirty,
        } = self
        else {
            return;
        };

        if !*hash_dirty {
            return;
        }

        rayon::join(
            || left.recalculate_hashes(cache),
            || right.recalculate_hashes(cache),
        );

        *hash = cache.hash(left.hash(), right.hash());
        *hash_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use proptest::prop_assume;
    use test_strategy::proptest;

    use crate::{Batch, Tree};

    #[proptest]
    fn root_hash_with_matches_insert(mut tree: Tree<16, i32>, batch: Batch<16, i32>) {
        let hash_with = tree.root_hash_with(&batch.elements().collect::<Vec<_>>());
        let result = tree.insert_batch(batch);

        prop_assume!(result.is_ok());

        assert_eq!(tree.root_hash(), hash_with);
    }
}
