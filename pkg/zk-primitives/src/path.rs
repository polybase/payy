use crate::{hash_merge, Element};

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
///  - `C` is [`hash_merge(hash_merge(0, 1), hash_merge(2, 3))`][crate::hash_merge] (i.e. the root
///  hash of the tree)
///
/// If you wanted to prove that `2` was in the tree with this function, you would do the
/// following:
/// ```rust
/// # use zk_primitives::*;
/// // create the iterator of tuples and left/right bools
/// let a = hash_merge([Element::new(0), Element::new(1)]);
/// let b = hash_merge([Element::new(2), Element::new(3)]);
/// let c = hash_merge([a, b]);
///
/// let siblings = [
///   (
///     Element::new(3),
///     false,  // the sibling right, so this value is false
///   ),
///   (
///     a,
///     true,  // the sibling left, so this value is true
///   ),
/// ];
///
/// // we are trying to prove the existence of `2`, so we use this as the `leaf` parameter
/// let root_hash = compute_merkle_root(Element::new(2), siblings);
/// assert_eq!(root_hash, c);  // the hashes match, proving that `2` is in the tree
///
/// // It might be the case that the tree had `Element::NULL_HASH` at this location in the tree
/// let root_hash_if_null = compute_merkle_root(Element::NULL_HASH, siblings);
/// assert_ne!(root_hash_if_null, c);  // these aren't equal
/// ```
pub fn compute_merkle_root<I: IntoIterator<Item = (Element, bool)>>(
    mut leaf: Element,
    siblings: I,
) -> Element {
    for (sibling, bit) in siblings {
        match bit {
            // bit is 0, this element is on the left
            false => leaf = hash_merge([leaf, sibling]),

            // bit is 1, this element is on the right
            true => leaf = hash_merge([sibling, leaf]),
        }
    }

    leaf
}
