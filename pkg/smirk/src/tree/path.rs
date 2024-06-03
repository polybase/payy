use std::iter::zip;

use crate::{Element, Lsb, Tree};

use super::tree_repr::Node;

/// A Merkle path generated from a [`Tree`] with depth `DEPTH`
///
/// A Merkle path can be used to verify the presence/absence of an [`Element`] in a [`Tree`] with a
/// known root hash.
///
/// A `Path<N>` has `N - 1` siblings, which can be surprising
///
/// Smirk's [`Tree`] is *sparse*. Every element has a "slot" that can either be occupied by that
/// element, or by [`Element::NULL_HASH`]. The path is calculated by looking at the `DEPTH` least
/// significant bits of the elements: 0 means "left", 1 means "right".
///
/// If the depth of the tree is smaller than the number of bits of the key (256), collisions are
/// possible for [`Element`]s with the same `DEPTH` least significant bits but different upper
/// bits.
///
/// To get a [`Path`], generate it from a tree:
/// ```rust
/// # use smirk::*;
/// let tree: Tree<64, _> = smirk! { 1, 2, 3, 4, 5 };
///
/// // generate a path for the element 1
/// let path = tree.path_for(Element::ONE);
///
/// // the path stores the root hash of the tree from which it was created
/// assert_eq!(path.actual_root_hash(), tree.root_hash());
///
/// // you can compute "what the root hash would have been" for a given element
/// let hash_if_1 = path.compute_root_hash(Element::ONE);
/// assert_eq!(hash_if_1, tree.root_hash());  // 1 was actually present in the tree
///
/// let hash_if_null = path.compute_root_hash(Element::NULL_HASH);
/// assert_ne!(hash_if_null, tree.root_hash());
///
/// // we can create an element with the same lower bits but different upper bits
/// let collides_with_1 = Element::ONE + (Element::ONE << 100);
/// let hash_if_collision = path.compute_root_hash(collides_with_1);
/// // this still doesn't have the same root hash
/// assert_ne!(hash_if_collision, tree.root_hash());
/// ```
#[derive(Debug, Clone)]
pub struct Path<const DEPTH: usize> {
    /// The siblings of the element with the deepest siblings first
    ///
    /// The first N - 1 values are the siblings, and the last value is the element that created
    /// this [`Path`]
    ///
    /// Ideally, we would have 2 fields here:
    ///  - `siblings: [Element: {N - 1}]`
    ///  - `element: Element`
    /// Unfortunately, Rust doesn't yet support this. So we just squeeze them together and deal
    /// with it 🤷
    pub siblings: [Element; DEPTH],

    pub(crate) root_hash: Element,
}

impl<const DEPTH: usize> Path<DEPTH> {
    /// Get a slice of siblings in this path
    ///
    /// Note that a [`Tree<DEPTH>`] will generate a `Path<DEPTH>` (due to limitations in Rust's
    /// const generics), but a `Path<DEPTH>` has `DEPTH - 1` siblings
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree = Tree::<64, i32>::new();
    /// let path = tree.path_for(Element::ONE);
    /// assert_eq!(path.siblings_deepest_first().len(), 63);
    /// ```
    #[inline]
    #[must_use]
    pub fn siblings_deepest_first(&self) -> &[Element] {
        &self.siblings[0..(DEPTH - 1)]
    }

    /// The [`Element`] that this path proves the (non) existance of (i.e. the argument to
    /// [`Tree::path_for`])
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree = Tree::<64, ()>::default();
    /// let element = Element::new(1234);
    ///
    /// let path = tree.path_for(element);
    /// assert_eq!(path.element(), element);
    /// ```
    #[inline]
    #[must_use]
    pub fn element(&self) -> Element {
        *self.siblings.last().unwrap()
    }

    /// The bits that are used by this path to determine left/right choices
    ///
    /// This returns the `DEPTH - 1` least significant bits, with the *most* significant bits first
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree = Tree::<4, i32>::default();
    /// let path = tree.path_for(Element::ONE);
    /// let bits: Vec<bool> = path.lsb().iter().copied().collect();
    ///
    /// // 1 is 0b00001
    /// // A tree of depth 4 means we need the 3 least significant bits
    /// assert_eq!(bits, vec![false, false, true])
    ///
    /// ```
    #[inline]
    #[must_use]
    #[doc(alias = "least_significant_bits")]
    pub fn lsb(&self) -> Lsb {
        self.element().lsb(DEPTH - 1)
    }

    /// Check whether this [`Path`] proves the existance of the given [`Element`]
    ///
    /// This is a small helper that simply compares the output of [`Self::compute_root_hash`] and
    /// [`Self::actual_root_hash`]
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree: Tree<64, _> = smirk! { 1, 2, 3 };
    ///
    /// let path_for_1 = tree.path_for(Element::new(1));
    /// let path_for_4 = tree.path_for(Element::new(4));
    ///
    /// assert_eq!(path_for_1.proves(Element::new(1)), true);
    /// assert_eq!(path_for_1.proves(Element::new(4)), false);
    /// assert_eq!(path_for_1.proves(Element::NULL_HASH), false);
    ///
    /// assert_eq!(path_for_4.proves(Element::new(1)), false);
    /// assert_eq!(path_for_4.proves(Element::new(4)), false);
    /// assert_eq!(path_for_4.proves(Element::NULL_HASH), true);
    /// ```
    #[inline]
    #[must_use]
    pub fn proves(&self, element: Element) -> bool {
        self.compute_root_hash(element) == self.actual_root_hash()
    }

    /// Compute the root hash of the tree from this path, with the given element in the
    /// corresponding slot
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree: Tree<64, ()> = smirk! { 1, 2, 3, 4, 5 };
    /// let element = Element::new(3);
    ///
    /// let path = tree.path_for(element);
    ///
    /// let computed_root = path.compute_root_hash(element);
    /// assert_eq!(computed_root, tree.root_hash());
    ///
    /// // if we use the null hash instead, the root hash will be different
    /// let root_hash_if_null = path.compute_root_hash(Element::NULL_HASH);
    /// assert_ne!(root_hash_if_null, tree.root_hash());
    /// ```
    ///
    /// Internally, this function calls [`zk_primitives::compute_merkle_root`]. See the docs for
    /// that function for more details
    #[must_use]
    pub fn compute_root_hash(&self, element: Element) -> Element {
        // `.lsb()` yields bits in *big endian* order - so we need to reverse them
        let bits = self.lsb().into_iter().rev();
        let siblings = self.siblings_deepest_first().iter().copied();

        zk_primitives::compute_merkle_root(element, zip(siblings, bits))
    }

    /// The root hash of the tree when this path was created
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree: Tree<64, _> = smirk! { 1, 2, 3, 4, 5 };
    /// let path = tree.path_for(Element::ONE);
    ///
    /// assert_eq!(tree.root_hash(), path.actual_root_hash());
    /// ```
    #[inline]
    #[must_use]
    pub fn actual_root_hash(&self) -> Element {
        self.root_hash
    }
}

impl<const DEPTH: usize, V, C> Tree<DEPTH, V, C> {
    /// Generate a [`Path`] that proves the presence/absence of a particular value at a location in
    /// the tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree: Tree<64, _> = smirk! {
    ///     1 => 123,
    ///     2 => 234,
    /// };
    ///
    /// let path = tree.path_for(Element::new(1));
    ///
    /// // if we calculate the root hash with the value `1`, the root hash will match the tree
    /// assert_eq!(path.compute_root_hash(Element::new(1)), tree.root_hash());
    /// ```
    ///
    /// Note that the last `DEPTH` bits of `element` determines the element's location in the tree.
    /// When we traverse to this location, we will either find:
    ///  - an element with the right least significant bits
    ///  - [`Element::NULL_HASH`]
    ///
    /// Given that, this function cannot fail, since every location is conceptually occupied
    /// (either with a real value or [`Element::NULL_HASH`])
    #[must_use]
    pub fn path_for(&self, element: Element) -> Path<DEPTH> {
        let bits = element.lsb(DEPTH - 1);

        let mut siblings = [Element::NULL_HASH; DEPTH];
        let mut tree = &self.tree;

        for (index, bit) in bits.iter().enumerate() {
            match tree {
                Node::Parent { left, right, .. } => match *bit {
                    // the bit is 0, so we follow the left hash, so right is the sibling
                    false => {
                        siblings[index] = right.hash();
                        tree = left;
                    }

                    // the bit is 1, so we follow the right hash, so left is the sibling
                    true => {
                        siblings[index] = left.hash();
                        tree = right;
                    }
                },
                // if we hit an empty node, we can simply continue in place
                //
                // a depth of `n` corresponds to `n - 1` left/right decisions, so we need to insert
                // `n - 1` elements into the siblings array
                Node::Empty { depth } => {
                    // we don't want to include `depth` here, because it was included when we
                    // calculated the parent (or root hash if this is the root of the tree)
                    for (i, depth) in (1..*depth).rev().enumerate() {
                        siblings[index + i] = Node::Empty { depth }.hash();
                    }

                    break;
                }
                Node::Leaf(_) => panic!("uh oh"),
            }
        }

        // set the last element
        *siblings.last_mut().unwrap() = element;

        // reverse the siblings so they are in depth-first order
        siblings[0..DEPTH - 1].reverse();

        Path {
            siblings,
            root_hash: self.root_hash(),
        }
    }
}

#[cfg(test)]
mod tests {

    use test_strategy::proptest;

    use super::*;

    #[proptest]
    fn cached_root_hash_is_correct(tree: Tree<64, i32>, element: Element) {
        let path = tree.path_for(element);
        assert_eq!(path.actual_root_hash(), tree.root_hash());
    }

    #[proptest]
    fn calculated_root_hash_is_correct(tree: Tree<64, i32>, element: Element) {
        let path = tree.path_for(element);

        let (correct, incorrect) = match tree.contains_element(element) {
            true => (element, Element::NULL_HASH),
            false => (Element::NULL_HASH, element),
        };

        let computed_root = path.compute_root_hash(correct);

        assert_eq!(computed_root, tree.root_hash());
        assert_eq!(computed_root, path.actual_root_hash());

        let incorrect_root = path.compute_root_hash(incorrect);
        assert_ne!(incorrect_root, tree.root_hash());
    }

    #[test]
    fn simple_path_example() {
        let mut tree = Tree::<64, i32>::new();
        tree.insert(Element::new(1), 1).unwrap();

        let path = tree.path_for(Element::new(1));
        assert_eq!(path.actual_root_hash(), tree.root_hash());

        let computed_root = path.compute_root_hash(Element::new(1));
        assert_eq!(computed_root, tree.root_hash());

        let computed_root = path.compute_root_hash(Element::NULL_HASH);
        assert_ne!(computed_root, tree.root_hash());
    }

    #[proptest]
    fn any_path_has_n_minus_one_siblings(tree: Tree<64, i32>, element: Element) {
        let path = tree.path_for(element);
        assert_eq!(path.siblings_deepest_first().len(), 63);
    }

    #[proptest]
    fn lsb_and_siblings_same_size(tree: Tree<16, i32>, element: Element) {
        let path = tree.path_for(element);
        assert_eq!(path.lsb().len(), path.siblings_deepest_first().len());
    }
}
