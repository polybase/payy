// lint-long-file-override allow-max-lines=400
use bitvec::{prelude::Msb0, vec::BitVec};

use crate::{Collision, hash::empty_tree_hash, hash_cache::HashCache};
use element::Element;

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

#[derive(Debug, Clone, Copy)]
pub enum Change {
    Insert,
    Remove,
}

impl Node {
    /// Calculates the root hash as if the given insertions and removals were applied.
    /// Does not modify the actual tree node.
    pub fn hash_with<const DEPTH: usize, C: HashCache>(
        &self,
        cache: &C,
        insert_elements: &[Element],
        remove_elements: &[Element],
    ) -> Element {
        // Combine insertions and removals into a single list with change type
        let mut changes_with_bits: Vec<((Change, Element), BitVec<u8, Msb0>)> = insert_elements
            .iter()
            .map(|&e| ((Change::Insert, e), e.lsb(DEPTH - 1).to_bitvec()))
            .collect::<Vec<_>>();

        changes_with_bits.extend(
            remove_elements
                .iter()
                .map(|&e| ((Change::Remove, e), e.lsb(DEPTH - 1).to_bitvec())),
        );

        // Sort based on the LSB path. This is crucial for correct processing.
        changes_with_bits.sort_unstable_by(|(_, a_bits), (_, b_bits)| a_bits.cmp(b_bits));

        // Separate back into changes and bits vectors
        let (changes, bits): (Vec<_>, Vec<_>) = changes_with_bits.into_iter().unzip();

        // Start the recursive hash calculation
        self.hash_with_inner::<DEPTH, C>(cache, &changes, &bits, 0)
    }

    /// Recursive helper for `hash_with`.
    #[allow(clippy::too_many_lines)]
    fn hash_with_inner<const DEPTH: usize, C: HashCache>(
        &self,
        cache: &C,
        changes: &[(Change, Element)], // Sorted list of changes (insert/remove, element)
        bits: &[BitVec<u8, Msb0>],     // Corresponding sorted LSB paths
        path_depth: usize,
    ) -> Element {
        if changes.is_empty() {
            // No pending updates for this subtree, so the cached hash is still valid unless this
            // node was marked dirty. When dirty, we must recompute to avoid returning a stale
            // cached value.
            if let Self::Parent {
                left,
                right,
                hash_dirty,
                ..
            } = self
                && *hash_dirty
            {
                let left_hash =
                    left.hash_with_inner::<DEPTH, C>(cache, changes, bits, path_depth + 1);
                let right_hash =
                    right.hash_with_inner::<DEPTH, C>(cache, changes, bits, path_depth + 1);

                return cache.hash(left_hash, right_hash);
            }

            return self.hash();
        }

        match self {
            Self::Leaf(current_element) => {
                // Check if any change affects *this specific leaf path*.
                // Since changes are sorted by path, only the first change (if any)
                // could possibly affect this leaf.
                assert!(
                    changes.len() <= 1,
                    "Logic error: Multiple changes target the same leaf path in hash_with_inner"
                );

                match changes.first() {
                    Some((Change::Insert, new_element)) => {
                        // An insert targets this leaf's path. Assume it replaces the current one.
                        // Collision check (LSBs must match) should ideally happen before.
                        debug_assert_eq!(
                            current_element.lsb(DEPTH - 1),
                            new_element.lsb(DEPTH - 1),
                            "LSB mismatch at leaf during hash_with insert simulation"
                        );
                        *new_element // Hash reflects the element *after* the change
                    }
                    Some((Change::Remove, removed_element)) => {
                        // A remove targets this leaf's path.
                        // Check if it's removing the element currently here.
                        if current_element == removed_element {
                            empty_tree_hash(1) // Removed, so becomes empty
                        } else {
                            // Attempting to remove a different element than what's present.
                            // This indicates either an invalid removal request (should be checked before)
                            // or the removal targets an element that *was* here but got replaced
                            // by an earlier insert in the same batch (complex case).
                            // For hash_with, we assume the state *after* all changes. If this leaf
                            // wasn't the one targeted for removal, it remains.
                            *current_element
                        }
                    }
                    None => *current_element, // No change affecting this leaf path
                }
            }
            Self::Parent { left, right, .. } => {
                // Find the split point for left/right based on the current bit in the path
                let right_start = bits
                    .iter()
                    .position(|b| b[path_depth]) // Find first path going right (bit == 1)
                    .unwrap_or(bits.len()); // If none go right, all go left

                // Slice the changes and bits for left and right subtrees
                let left_bits = &bits[..right_start];
                let left_changes = &changes[..right_start];
                let right_bits = &bits[right_start..];
                let right_changes = &changes[right_start..];

                // Recursively calculate hashes for left and right children
                let left_hash = left.hash_with_inner::<DEPTH, C>(
                    cache,
                    left_changes,
                    left_bits,
                    path_depth + 1,
                );
                let right_hash = right.hash_with_inner::<DEPTH, C>(
                    cache,
                    right_changes,
                    right_bits,
                    path_depth + 1,
                );

                // Merge the resulting hashes
                cache.hash(left_hash, right_hash)
            }
            Self::Empty { depth: 1 } => {
                // Base case: An empty node at the leaf depth.
                assert!(
                    changes.len() <= 1,
                    "Logic error: Multiple changes target the same empty leaf path: {:?}",
                    &changes,
                );
                match changes.first() {
                    Some((Change::Insert, element)) => *element, // Insert creates a leaf
                    Some((Change::Remove, _)) | None => empty_tree_hash(1), // Removing from empty or no change -> stays empty
                }
            }
            Self::Empty { depth } => {
                // An empty node at an intermediate depth.
                // If no changes pass through this node, its hash remains the empty tree hash.
                if changes.is_empty() {
                    return empty_tree_hash(*depth);
                }

                // Otherwise, we need to simulate its expansion and recurse.
                // Find the split point for left/right changes.
                let right_start = bits
                    .iter()
                    .position(|b| b[path_depth])
                    .unwrap_or(bits.len());
                let left_bits = &bits[..right_start];
                let left_changes = &changes[..right_start];
                let right_bits = &bits[right_start..];
                let right_changes = &changes[right_start..];

                // Simulate the two empty children it would have.
                let temp_left_node = Node::Empty { depth: *depth - 1 };
                let temp_right_node = Node::Empty { depth: *depth - 1 };

                // Recurse into the simulated children.
                let left_hash = temp_left_node.hash_with_inner::<DEPTH, C>(
                    cache,
                    left_changes,
                    left_bits,
                    path_depth + 1,
                );
                let right_hash = temp_right_node.hash_with_inner::<DEPTH, C>(
                    cache,
                    right_changes,
                    right_bits,
                    path_depth + 1,
                );

                // Merge the results.
                cache.hash(left_hash, right_hash)
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
    ///
    /// The elements and bits should be sorted by the bits before calling this function
    pub(crate) fn insert_without_hashing<const N: usize>(
        &mut self,
        elements: &[(Change, Element)],
        bits: &[BitVec<u8, Msb0>],
        path_depth: usize,
    ) -> Result<bool, Collision> {
        match self {
            Self::Leaf(e) if elements.iter().any(|(_, ee)| e == ee) => {
                let Some((change, _)) = elements.iter().find(|(_, ee)| e == ee) else {
                    unreachable!()
                };

                match change {
                    Change::Insert => {
                        // It's already in tree
                        Ok(false)
                    }
                    Change::Remove => {
                        *self = Self::Empty { depth: 1 };
                        Ok(true)
                    }
                }
            }
            Self::Leaf(e)
                if bits.iter().any({
                    let e_lsb = e.lsb(N - 1);
                    move |b| b == &e_lsb[..]
                }) =>
            {
                Err(Collision {
                    in_tree: *e,
                    inserted: elements
                        .iter()
                        .find(|(_, ee)| e.lsb(N - 1) == ee.lsb(N - 1))
                        .unwrap()
                        .1,
                    depth: N,
                    struct_name: StructName::Tree,
                })
            }
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
                let rights_start = bits
                    .iter()
                    .position(|b| b[path_depth])
                    .unwrap_or(bits.len());
                let lefts = &bits[..rights_start];
                let rights = &bits[rights_start..];
                let lefts_elements = &elements[..rights_start];
                let rights_elements = &elements[rights_start..];

                let (left, right) = match (lefts.is_empty(), rights.is_empty()) {
                    (true, true) => return Ok(false),
                    (false, true) => (
                        { left.insert_without_hashing::<N>(lefts_elements, lefts, path_depth + 1) },
                        Ok(false),
                    ),
                    (true, false) => (Ok(false), {
                        right.insert_without_hashing::<N>(rights_elements, rights, path_depth + 1)
                    }),
                    (false, false) => (
                        right.insert_without_hashing::<N>(rights_elements, rights, path_depth + 1),
                        left.insert_without_hashing::<N>(lefts_elements, lefts, path_depth + 1),
                    ),
                };

                *hash_dirty = matches!(right, Ok(true)) || matches!(left, Ok(true));

                right?;
                left?;

                Ok(*hash_dirty)
            }
            Self::Empty { depth: 1 } => {
                assert_eq!(elements.len(), 1);
                match elements.first().copied().unwrap() {
                    (Change::Insert, e) => {
                        *self = Self::Leaf(e);
                        Ok(true)
                    }
                    (Change::Remove, _) => Ok(false),
                }
            }

            Self::Empty { depth } => {
                // split an empty tree into two empty subtrees
                *self = Self::Parent {
                    left: Box::new(Self::Empty { depth: *depth - 1 }),
                    right: Box::new(Self::Empty { depth: *depth - 1 }),
                    hash: empty_tree_hash(*depth),
                    hash_dirty: false,
                };

                // now try again
                self.insert_without_hashing::<N>(elements, bits, path_depth)
            }
        }
    }

    pub fn recalculate_hashes<C: HashCache>(
        &mut self,
        cache: &C,
        hash_remove_callback: &(impl Fn((&Element, &Element)) + Send + Sync),
        hash_set_callback: &(impl Fn((&Element, &Element, &Element)) + Send + Sync),
    ) {
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

        let left_hash_before = left.hash();
        let right_hash_before = right.hash();
        hash_remove_callback((&left_hash_before, &right_hash_before));

        rayon::join(
            || left.recalculate_hashes(cache, hash_remove_callback, hash_set_callback),
            || right.recalculate_hashes(cache, hash_remove_callback, hash_set_callback),
        );

        let left_hash = left.hash();
        let right_hash = right.hash();
        *hash = cache.hash(left_hash, right_hash);
        *hash_dirty = false;
        hash_set_callback((&left_hash, &right_hash, hash));
    }
}

#[cfg(test)]
mod tests {
    use proptest::prop_assume;
    use test_strategy::proptest;

    use crate::{Batch, Tree};

    #[proptest]
    fn root_hash_with_matches_insert(mut tree: Tree<16, i32>, batch: Batch<16, i32>) {
        let hash_with = tree.root_hash_with(&batch.insert_elements().collect::<Vec<_>>(), &[]);
        let result = tree.insert_batch(batch, |_| {}, |_| {});

        prop_assume!(result.is_ok());

        assert_eq!(tree.root_hash(), hash_with);
    }
}
