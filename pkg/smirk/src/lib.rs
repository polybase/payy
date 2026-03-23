#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::explicit_deref_methods)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::single_match_else)]
#![allow(clippy::from_iter_instead_of_collect)]
#![deny(missing_docs)]

//! # Smirk (**S**parse **M**e**RK**le tree)
//!
//! A sparse Merkle [`Tree`], optimized for use with the Halo2 proof system.
//!
//! Conceptually, a [`Tree`] is quite similar to a `HashMap<Element, V>`, where [`Element`] is a
//! 256-bit unsigned integer. The API of [`Tree`] reflects this similarity.
//!
//! ```rust
//! # use smirk::*;
//! # use element::Element;
//! // the tree is generic over the depth, 64 is a good default
//! let mut tree = Tree::<64, String>::new();
//!
//! // `Element`s are 256-bit integers which act as keys
//! tree.insert(Element::new(1), String::from("hello"));
//! tree.insert(Element::new(2), String::from("world"));
//!
//! // we can get the root hash of the tree
//! println!("{}", tree.root_hash());
//! ```
//! ## Root hash
//!
//! Like all Merkle trees, a [`Tree`] has a [`root hash`][Tree::root_hash]. Smirk is a binary tree,
//! and so the hash of a node is calculated by taking the hashes of the left and right children,
//! concatenating them, and then hashing the result. The resulting structure ensures that if any of
//! the [`Element`]s in the tree change, the root hash will change.
//!
//! Smirk trees have a deterministic structure, meaning that the root hash for any set of elements
//! is always the same, regardless of insertion order. There is no rebalancing, or anything else
//! that could affect the root hash without changing the set of inserted elements.
//!
//! Note that root hash is determined entirely by the set of **elements**, and not the set of
//! **entries**. This means that calling `tree.insert(Element::new(1), 123)` and
//! `tree.insert(Element::new(1), 234)` will cause the **same** change in the root hash.
//! This is because Smirk is optimized for use with the Halo2 proof system, which is quite rigid,
//! and it's challenging to write generic code that can integrate with Halo2 circuits.
//!
//! As such, the root hash only guarantees the integrity of the set of elements. Guarantees about
//! the truthfulness of the values must be established through other mechanisms (i.e. cryptographic
//! signatures, consensus, etc.).
//!
//! ## Collisions
//!
//! A Smirk tree is represented by a binary tree with a fixed depth controlled by a const generic
//! parameter `DEPTH`. Each node of depth `DEPTH` have two children of depth `DEPTH - 1`, except
//! for the case of a node with depth `1`, which either contains an [`Element`], or is empty, which
//! is represented by [`element::Element::NULL_HASH`].
//!
//! However, especially if `DEPTH` is small, there may be two elements with the same `DEPTH - 1`
//! least significant bits, meaning they would occupy the same slot. This is a collision, and the
//! second [`Element`] will fail to insert.
//!
//! Because [`element::Element::NULL_HASH`] is a valid value at any point in the tree, it is considered to
//! collide with every value, and cannot be inserted into the tree ever.
//!
//! [`Element`]: element::Element

/// APIs relating to batched inserts into [`Tree`]s and [`Persistent`]s
///
/// [`Persistent`]: crate::storage::Persistent
/// [`Tree`]: crate::Tree
mod batch;
mod hash;
/// Caching of hash values
pub mod hash_cache;
mod macros;
/// APIs relating to persistence of a [`Tree`]
pub mod storage;
mod tree;

pub use batch::Batch;
pub use hash::empty_tree_hash;
pub use tree::{Collision, CollisionError, Path, Tree};
