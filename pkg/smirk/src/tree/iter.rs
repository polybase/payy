use crate::Tree;
use element::Element;
use std::collections::btree_map;

#[derive(Debug, Clone)]
pub struct Elements<'a, V> {
    inner: btree_map::Iter<'a, Element, V>,
}

/// Backwards-compatible alias for the historical iterator type.
pub type Iter<'a, V> = Elements<'a, V>;

impl<'a, V> Iterator for Elements<'a, V> {
    type Item = (&'a Element, &'a V);

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
    /// # use element::Element;
    /// let tree: Tree<64, _> = smirk! { 1, 2, 3 };
    ///
    /// let vec: Vec<(&Element, &())> = tree.elements().collect();
    ///
    /// assert_eq!(vec, vec![
    ///   (&Element::new(1), &()),
    ///   (&Element::new(2), &()),
    ///   (&Element::new(3), &()),
    /// ]);
    /// ```
    #[inline]
    #[must_use]
    #[doc(alias = "iter")]
    pub fn elements(&self) -> Iter<'_, V> {
        let inner = self.entries.iter();
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
