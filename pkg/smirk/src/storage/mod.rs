use core::fmt::Debug;
use std::path::Path;

use borsh::{BorshDeserialize, BorshSerialize};
use rocksdb::DB;

pub use error::Error;

use crate::{hash_cache::SimpleHashCache, Element, Tree};

mod batch;
mod error;
mod format;
mod load;
mod store;

#[cfg(test)]
mod tests;

/// A wrapper around [`Tree`] that persists data to a rocksdb instance
///
/// ```rust
/// # use smirk::*;
/// # use smirk::storage::*;
/// # let dir = tempdir::TempDir::new("smirk_doctest").unwrap();
/// # let path = dir.path().join("db");
/// ```
pub struct Persistent<const DEPTH: usize, V> {
    tree: Tree<DEPTH, V, SimpleHashCache>,
    db: DB,
}

impl<const DEPTH: usize, V> Persistent<DEPTH, V> {
    /// Create a new, empty [`Persistent`] [`Tree`] backed by a rocksdb instance at `path`
    ///
    /// ```rust
    /// # use smirk::*;
    /// # use smirk::storage::*;
    /// # let dir = tempdir::TempDir::new("smirk_doctest").unwrap();
    /// # let path = dir.path().join("db");
    /// let mut persistent = Persistent::<64, i32>::new(&path).unwrap();
    ///
    /// persistent.insert(Element::ONE, 123).unwrap();
    /// println!("{}", persistent.tree().root_hash());
    /// ```
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let db = DB::open_default(path)?;
        let tree = Tree::new();

        Ok(Self { tree, db })
    }

    /// Load a [`Persistent`] [`Tree`] from a rocksdb database located at `path`
    ///
    /// ```rust
    /// # use smirk::*;
    /// # use smirk::storage::*;
    /// # let dir = tempdir::TempDir::new("smirk_doctest").unwrap();
    /// # let path = dir.path().join("db");
    /// let mut persistent = Persistent::<64, i32>::new(&path).unwrap();
    /// persistent.insert(Element::ONE, 123).unwrap();
    ///
    /// drop(persistent);
    ///
    /// // now load the tree again
    /// let persistent = Persistent::<64, i32>::load(&path).unwrap();
    /// assert_eq!(persistent.tree().get(Element::ONE), Some(&123));
    /// ```
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error>
    where
        V: BorshDeserialize + BorshSerialize + Debug + Clone + Send + Sync + 'static,
    {
        let db = DB::open_default(path)?;
        let tree = load::load_tree(&db)?;

        Ok(Self { tree, db })
    }

    /// Get a reference to the wrapped tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// # use smirk::storage::*;
    /// # let dir = tempdir::TempDir::new("smirk_doctest").unwrap();
    /// # let path = dir.path().join("db");
    /// let persistent = Persistent::<64, i32>::new(&path).unwrap();
    ///
    /// let tree = persistent.tree();
    ///
    /// assert_eq!(tree.root_hash(), Tree::<64, i32>::new().root_hash());
    /// ```
    #[inline]
    #[must_use]
    pub fn tree(&self) -> &Tree<DEPTH, V, SimpleHashCache> {
        &self.tree
    }

    /// Get a reference to the rocksdb instance
    #[inline]
    #[must_use]
    pub fn db(&self) -> &DB {
        &self.db
    }

    /// Split this instance into the [`Tree`] and [`DB`] that make up this [`Persistent`]
    ///
    /// Since [`Persistent`] doesn't provide any way to get a `&mut Tree`, this is the only way to
    /// get mutable access to the inner tree
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (Tree<DEPTH, V, SimpleHashCache>, DB) {
        let Self { tree, db } = self;
        (tree, db)
    }

    /// Insert an element into the in-memory tree, and persist the element to the backing rocksdb
    /// store
    ///
    /// ```rust
    /// # use smirk::*;
    /// # use smirk::storage::*;
    /// # let dir = tempdir::TempDir::new("smirk_doctest").unwrap();
    /// # let path = dir.path().join("db");
    /// let mut persistent = Persistent::<64, ()>::new(&path).unwrap();
    ///
    /// persistent.insert(Element::new(1), ()).unwrap();
    /// persistent.insert(Element::new(2), ()).unwrap();
    /// persistent.insert(Element::new(3), ()).unwrap();
    ///
    /// // attempts to insert collidintg elements will fail
    /// let collides = Element::new(1) + (Element::new(1) << 100);
    /// let _err = persistent.insert(collides, ()).unwrap_err();
    /// ```
    /// Note that this function calls [`Tree::insert`], so inherits the performance characteristics
    /// of that function. If you are inserting many elements, and want to avoid recalculating
    /// hashes unnecessarily, use [`Persistent::insert_batch`]
    /// instead
    pub fn insert(&mut self, element: Element, value: V) -> Result<(), Error>
    where
        V: BorshSerialize + BorshDeserialize + Send + Sync + 'static + Clone,
    {
        self.insert_batch(crate::batch! { element => value })
    }

    /// Store all computed hashes from the in-memory tree into rocksdb
    ///
    /// Note that this function is never called automatically when inserting. Make sure to call
    /// this function, otherwise no precomputed hashes will be persisted
    pub fn persist_hashes(&self) -> Result<(), Error>
    where
        V: BorshSerialize + BorshDeserialize + Send + Sync + 'static + Clone,
    {
        store::synchronize_hashes(&self.db, &self.tree)
    }
}
