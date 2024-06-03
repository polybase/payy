use crate::{hash_cache::NoopHashCache, Element};
use std::collections::BTreeMap;

mod batch;
mod error;
mod insert;
mod iter;
mod known_hashes;
mod path;
mod raw_api;
mod tree_repr;

use bitvec::vec::BitVec;
pub use error::{Collision, CollisionError};
pub use iter::{Elements, IntoIter, Iter};
pub use path::Path;

pub(crate) use error::StructName;

#[cfg(any(test, feature = "proptest"))]
pub mod proptest;

/// A sparse Merkle tree
///
/// Conceptually, this type is roughly equivalent to a `HashMap<Element, V>`, and the API reflects
/// this:
///
/// ```rust
/// # use smirk::*;
/// let mut tree = Tree::<64, i32>::new();
///
/// tree.insert(Element::new(1), 123);
/// tree.insert(Element::new(2), 234);
/// tree.insert(Element::new(3), 345);
///
/// assert!(tree.contains_element(Element::new(1)));
///
/// for (element, value) in tree.iter() {
///     println!("the tree contains {value} at element {element}");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Tree<const DEPTH: usize, V, C = NoopHashCache> {
    /// The tree-like representation
    tree: tree_repr::Node,
    entries: BTreeMap<Element, V>,
    cache: C,
}

impl<const DEPTH: usize, V, C> PartialEq for Tree<DEPTH, V, C> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.root_hash() == other.root_hash()
    }
}

impl<const DEPTH: usize, V, C> Eq for Tree<DEPTH, V, C> {}

impl<const DEPTH: usize, V, C> Default for Tree<DEPTH, V, C>
where
    C: Default,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const DEPTH: usize, V, C> Tree<DEPTH, V, C> {
    /// Creates a new, empty tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree = Tree::<64, i32>::new();
    /// assert!(tree.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Self
    where
        C: Default,
    {
        Self {
            entries: BTreeMap::new(),
            tree: tree_repr::Node::Empty { depth: DEPTH },
            cache: C::default(),
        }
    }

    /// Creates a new, empty tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree = Tree::<64, i32>::new();
    /// assert!(tree.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn new_with_cache(cache: C) -> Self {
        Self {
            entries: BTreeMap::new(),
            tree: tree_repr::Node::Empty { depth: DEPTH },
            cache,
        }
    }

    /// Get access to the inner cache of this tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// # use smirk::hash_cache::*;
    /// let tree = Tree::<64, i32, NoopHashCache>::new();
    /// let cache = tree.cache();
    /// let hash = cache.hash(Element::new(1), Element::new(2));
    /// println!("{hash}");
    /// ```
    #[inline]
    #[must_use]
    pub fn cache(&self) -> &C {
        &self.cache
    }

    /// The number of elements stored in this tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// let mut tree = Tree::<64, i32>::new();
    ///
    /// assert_eq!(tree.len(), 0);
    ///
    /// tree.insert(Element::new(1), 123);
    /// assert_eq!(tree.len(), 1);
    ///
    /// tree.insert(Element::new(100), 234);
    /// assert_eq!(tree.len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether this tree contains no elements
    ///
    /// ```rust
    /// # use smirk::*;
    /// let mut tree = Tree::<64, i32>::new();
    ///
    /// assert_eq!(tree.is_empty(), true);
    ///
    /// tree.insert(Element::new(1), 123);
    ///
    /// assert_eq!(tree.is_empty(), false);
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns `true` if the tree contains a given element
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree: Tree<64, _> = smirk! { 1, 2, 3 };
    ///
    /// assert_eq!(tree.contains_element(Element::new(1)), true);
    /// assert_eq!(tree.contains_element(Element::new(2)), true);
    /// assert_eq!(tree.contains_element(Element::new(3)), true);
    /// assert_eq!(tree.contains_element(Element::new(4)), false);
    /// ```
    #[inline]
    #[must_use]
    pub fn contains_element(&self, element: Element) -> bool {
        self.entries.contains_key(&element)
    }

    /// The root hash of the tree
    ///
    /// This value represents every value contained in the tree, i.e. any changes to the tree will
    /// change the root hash
    ///
    /// ```rust
    /// # use smirk::*;
    /// let mut tree = Tree::<64, i32>::new();
    /// let hash_1 = tree.root_hash();
    ///
    /// tree.insert(Element::new(1), 123);
    /// let hash_2 = tree.root_hash();
    ///
    /// tree.insert(Element::new(2), 234);
    /// let hash_3 = tree.root_hash();
    ///
    /// assert_ne!(hash_1, hash_2);
    /// assert_ne!(hash_1, hash_3);
    /// assert_ne!(hash_2, hash_3);
    /// ```
    /// This value is cached internally, so calls to this function are essentially free
    #[inline]
    #[must_use]
    pub fn root_hash(&self) -> Element {
        self.tree.hash()
    }

    /// Compute what the root hash would be if all of `extra_elements` were inserted
    ///
    /// ```rust
    /// # use smirk::*;
    /// let mut tree = Tree::<64, ()>::new();
    /// let hash_with_1 = tree.root_hash_with(&[Element::new(1)]);
    ///
    /// tree.insert(Element::new(1), ());
    ///
    /// assert_eq!(hash_with_1, tree.root_hash());
    /// ```
    #[inline]
    #[must_use]
    pub fn root_hash_with(&self, extra_elements: &[Element]) -> Element {
        self.tree.hash_with::<DEPTH>(extra_elements, &BitVec::new())
    }
}
