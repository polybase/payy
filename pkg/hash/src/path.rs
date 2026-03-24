use crate::{Element, hash_merge};
use core::iter::zip;

/// Compute the root hash of a merkle tree
///
/// `siblings` is an [`Iterator`] that yields tuples containing the sibling, and a boolean value
/// that indicates whether the sibling in question was on the left or right (`false` means that the
/// sibling is on the right, `true` means that the sibling is on the left).
///
/// The elements of `siblings` are in "deepest-first" order.
/// Note that the root hash of the tree is not considered to be a sibling, so a tree of depth `N`
/// would have `N - 1` siblings
///
/// For example, consider the following tree:
/// ```text
///          ┌─────┐
///          │  C  │
///          └──┬──┘
///             │
///       ┌─────┴─────┐
///       │           │
///    ┌──▼──┐     ┌──▼──┐
///    │  A  │     │  B  │
///    └──┬──┘     └──┬──┘
///       │           │
///    ┌──┴──┐     ┌──┴──┐
///    │     │     │     │
///  ┌─▼─┐ ┌─▼─┐ ┌─▼─┐ ┌─▼─┐
///  │ 0 │ │ 1 │ │ 2 │ │ 3 │
///  └───┘ └───┘ └───┘ └───┘
/// ```
/// Here:
///  - `A` is [`hash_merge(0, 1)`][crate::hash_merge]
///  - `B` is [`hash_merge(2, 3)`][crate::hash_merge]
///  - `C` is [`hash_merge(hash_merge(0, 1), hash_merge(2, 3))`][crate::hash_merge] (i.e. the root hash of the tree)
///
/// If you wanted to prove that `2` was in the tree with this function, you would do the
/// following:
/// ```rust
/// # use element::Element;
/// # use hash::*;
/// // create the iterator of tuples and left/right bools
/// let a = hash_merge([Element::new(0), Element::new(1)]);
/// let b = hash_merge([Element::new(2), Element::new(3)]);
/// let c = hash_merge([a, b]);
///
/// let siblings = [
///   Element::new(3),
///   a
/// ];
///
/// // we are trying to prove the existence of `2`, so we use this as the `leaf` parameter
/// let root_hash = compute_merkle_root(Element::new(2), Element::new(2), &siblings);
/// assert_eq!(root_hash, c);  // the hashes match, proving that `2` is in the tree
///
/// // It might be the case that the tree had `Element::NULL_HASH` at this location in the tree
/// let root_hash_if_null = compute_merkle_root(Element::NULL_HASH, Element::NULL_HASH, &siblings);
/// assert_ne!(root_hash_if_null, c);  // these aren't equal
/// ```
pub fn compute_merkle_root(leaf: Element, path_element: Element, siblings: &[Element]) -> Element {
    let bits = least_significant_bits(path_element, siblings.len());
    compute_merkle_root_from_iter(leaf, zip(siblings, bits))
}

/// Compute the root hash of a merkle tree, assuming null leaf.
///
/// This allows you to prove that an element is not in the tree
///
pub fn compute_null_root(leaf: Element, siblings: &[Element]) -> Element {
    let bits = least_significant_bits(leaf, siblings.len());
    compute_merkle_root_from_iter(Element::NULL_HASH, zip(siblings, bits))
}

fn compute_merkle_root_from_iter<'a, I: IntoIterator<Item = (&'a Element, bool)>>(
    mut leaf: Element,
    siblings: I,
) -> Element {
    for (sibling, bit) in siblings {
        match bit {
            // bit is 0, this element is on the left
            false => leaf = hash_merge([leaf, *sibling]),

            // bit is 1, this element is on the right
            true => leaf = hash_merge([*sibling, leaf]),
        }
    }

    leaf
}

/// Computes a new merkle path for `leaf` (given existing siblings).
///
/// This function is useful when you are inserting an element into a tree to calculate the new path
/// that proves the element is now in the tree.
///
pub fn compute_merkle_path_for_leaf(leaf: Element, siblings: &[Element]) -> Vec<Element> {
    let mut path = vec![leaf];
    let mut hash = leaf;

    let bits = least_significant_bits(leaf, siblings.len());

    for (sibling, bit) in zip(siblings, bits) {
        match bit {
            // bit is 0, this element is on the left
            false => hash = hash_merge([hash, *sibling]),

            // bit is 1, this element is on the right
            true => hash = hash_merge([*sibling, hash]),
        }
        path.push(hash);
    }

    path
}

/// The hash of an empty tree with a given depth
///
/// This function can be defined recursively:
///  - `empty_tree_hash(1) = Element::NULL_HASH`
///  - `empty_tree_hash(n) = hash_merge(empty_tree_hash(n - 1), empty_tree_hash(n - 1))`
///
/// # Panics
///
/// Panics if `depth` is 0, since there is no such thing as a tree with depth 0.
#[inline]
#[must_use]
pub fn empty_tree_root(depth: usize) -> Element {
    assert_ne!(depth, 0, "the smallest possible tree has depth 1");

    compute_merkle_root_from_iter(
        Element::NULL_HASH,
        (1..depth).map(|_| (&Element::NULL_HASH, false)),
    )
}

/// Get the least significant bits of an element (upto depth)
///
/// This function is useful for getting the bits of an element that are used to compute the merkle
/// path.
///
#[inline]
pub fn least_significant_bits(element: Element, depth: usize) -> impl Iterator<Item = bool> {
    element.lsb(depth).into_iter().rev()
}

// TODO: add tests for these!
