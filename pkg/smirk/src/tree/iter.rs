use std::{collections::btree_map, iter::Copied};

use crate::{Element, Tree};

#[derive(Debug, Clone)]
pub struct Elements<'a, V> {
    inner: Copied<btree_map::Keys<'a, Element, V>>,
}

impl<'a, V> Iterator for Elements<'a, V> {
    type Item = Element;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<const DEPTH: usize, V, C> Tree<DEPTH, V, C> {
    /// Get an iterator over the elements in this set
    ///
    /// Elements are yielded in ascending order. Note that this is not necessarily the same order
    /// as the left-to-right traversal of the tree, since the tree ordering is based on the least
    /// significant `DEPTH - 1` bits of the element
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree: Tree<64, _> = smirk! { 1, 2, 3 };
    ///
    /// let vec: Vec<Element> = tree.elements().collect();
    ///
    /// assert_eq!(vec, vec![
    ///   Element::new(1),
    ///   Element::new(2),
    ///   Element::new(3),
    /// ]);
    /// ```
    #[inline]
    #[must_use]
    #[doc(alias = "iter")]
    pub fn elements(&self) -> Elements<V> {
        let inner = self.entries.keys().copied();
        Elements { inner }
    }
}

#[derive(Debug)]
pub struct IntoIter<V> {
    inner: btree_map::IntoIter<Element, V>,
}

impl<V> Iterator for IntoIter<V> {
    type Item = (Element, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<const N: usize, V, C> IntoIterator for Tree<N, V, C> {
    type Item = (Element, V);
    type IntoIter = IntoIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        let inner = self.entries.into_iter();
        IntoIter { inner }
    }
}

/// An [`Iterator`] over [`Element`]s and values
#[derive(Debug, Clone)]
pub struct Iter<'a, V> {
    inner: btree_map::Iter<'a, Element, V>,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (&'a Element, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<const N: usize, V, C> Tree<N, V, C> {
    /// Get an iterator over elements and values
    #[must_use]
    pub fn iter(&self) -> Iter<V> {
        Iter {
            inner: self.entries.iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use test_strategy::proptest;

    use super::*;

    #[proptest]
    fn iter_and_into_iter() {
        let mut tree = Tree::<64, i32>::new();

        tree.insert(Element::new(1), 1).unwrap();
        tree.insert(Element::new(10), 10).unwrap();
        tree.insert(Element::new(5), 5).unwrap();
        tree.insert(Element::new(3), 3).unwrap();
        tree.insert(Element::new(111), 111).unwrap();

        let expected = [
            (Element::new(1), 1),
            (Element::new(3), 3),
            (Element::new(5), 5),
            (Element::new(10), 10),
            (Element::new(111), 111),
        ];

        let vec: Vec<_> = tree.into_iter().collect();
        assert_eq!(vec, expected);
    }
}
