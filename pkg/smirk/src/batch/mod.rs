use std::collections::HashSet;

use zk_primitives::{Element, Lsb};

use crate::{tree, Collision, CollisionError};

#[cfg(any(test, feature = "proptest"))]
mod proptest;

mod merge;

/// A batch of key-value pairs to insert into a [`Tree`]
///
/// This batch is generic over the depth of the tree that it is inserted into, since the tree depth
/// is required for calculating whether two [`Element`]s collide.
///
/// [`Tree`]: crate::Tree
#[derive(Debug, Clone)]
#[must_use = "a `Batch` does nothing unless inserted"]
pub struct Batch<const DEPTH: usize, V> {
    pub(crate) entries: Vec<(Element, V)>,
    /// The LSBs of the elements that have been inserted, for efficient checking of new entries
    pub(crate) lsbs: HashSet<Lsb>,
}

impl<const DEPTH: usize, V> Default for Batch<DEPTH, V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            lsbs: HashSet::new(),
        }
    }
}

impl<const DEPTH: usize, V> Batch<DEPTH, V> {
    /// Create a new, empty batch
    ///
    /// ```rust
    /// # use smirk::*;
    /// let mut batch = Batch::<64, String>::new();
    ///
    /// batch.insert(Element::new(1), String::from("hello"));
    /// batch.insert(Element::new(2), String::from("world"));
    /// ```
    ///
    /// Alternatively, you can use the [`batch!`] macro for a more concise syntax:
    /// ```rust
    /// # use smirk::*;
    /// let batch: Batch<64, _> = batch! {
    ///     1 => String::from("hello"),
    ///     2 => String::from("world"),
    /// };
    /// ```
    ///
    /// [`batch!`]: crate::batch!
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty [`Batch`] with at least the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            lsbs: HashSet::with_capacity(capacity),
        }
    }

    /// Check whether this batch is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert a key-value pair into this [`Batch`]
    ///
    /// This function will return `Err` if there is already an [`Element`] with the same [least
    /// significant bits]
    ///
    /// Note that, unlike [`Tree::insert`], no hashing takes place when inserting into a [`Batch`],
    /// so performance is very good
    ///
    /// [least significant bits]: zk_primitives::Lsb
    /// [`Tree::insert`]: crate::Tree::insert
    pub fn insert(&mut self, element: Element, value: V) -> Result<(), CollisionError> {
        let lsb = element.lsb(DEPTH - 1);

        if self.lsbs.contains(&lsb) {
            // unwrap is fine because we only run this if we found a collision above
            let in_tree = self.find_element_with_lsb(element.lsb(DEPTH - 1)).unwrap();

            let collision = Collision {
                in_tree,
                inserted: element,
                depth: DEPTH,
                struct_name: tree::StructName::Batch,
            };

            return Err(CollisionError {
                collisions: vec![collision],
            });
        }

        self.lsbs.insert(lsb);
        self.entries.push((element, value));

        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn remove(&mut self, element: Element) {
        let lsb = element.lsb(DEPTH - 1);

        self.entries.retain(|(e, _)| *e != element);
        self.lsbs.remove(&lsb);
    }

    /// Get an iterator over the elements that have been inserted into this [`Batch`]
    pub fn elements(&self) -> impl Iterator<Item = Element> + '_ {
        self.entries.iter().map(|(element, _)| element).copied()
    }

    /// Get an iterator over the values that have been inserted into this [`Batch`]
    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Get an iterator over the entries that have been inserted into this [`Batch`]
    pub fn entries(&self) -> impl Iterator<Item = &(Element, V)> + '_ {
        self.entries.iter()
    }

    pub(crate) fn find_element_with_lsb(&self, lsb: Lsb) -> Option<Element> {
        self.elements().find(|e| e.lsb(DEPTH - 1) == lsb)
    }

    /// Create a [`Batch`] from an [`Iterator`] over tuples of [`Element`]s and values
    ///
    /// ```rust
    /// # use smirk::*;
    /// let batch = Batch::<64, _>::from_entries([
    ///     (Element::new(1), 123),
    ///     (Element::new(2), 234),
    ///     (Element::new(3), 345),
    /// ]);
    /// ```
    pub fn from_entries<I>(entries: I) -> Result<Self, CollisionError>
    where
        I: IntoIterator<Item = (Element, V)>,
    {
        let mut batch = Self::new();

        for (element, value) in entries {
            batch.insert(element, value)?;
        }

        Ok(batch)
    }

    /// Create a [`Batch`] from an [`Iterator`] of [`Element`]s, using the [`Default`]
    /// implementation for the values
    ///
    /// ```rust
    /// # use smirk::*;
    /// let batch = Batch::<64, ()>::from_elements([
    ///     Element::new(1),
    ///     Element::new(2),
    ///     Element::new(3),
    /// ]);
    /// ```
    pub fn from_elements<I>(entries: I) -> Result<Self, CollisionError>
    where
        I: IntoIterator<Item = Element>,
        V: Default,
    {
        Self::from_entries(entries.into_iter().map(|element| (element, V::default())))
    }
}
