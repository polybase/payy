use crate::{batch, hash_cache::HashCache, CollisionError, Element, Path, Tree};

impl<const DEPTH: usize, V, C> Tree<DEPTH, V, C> {
    /// Insert a non-null element and a value into the tree
    ///
    /// Returns whether the value was newly inserted. That is:
    ///
    /// - If the tree did not previously contain an entry at this element, `true` is returned
    /// - If the tree already contained this an entry at this element, `false` is returned
    /// ```rust
    /// # use smirk::*;
    /// let mut tree = Tree::<64, i32>::new();
    ///
    /// let res = tree.insert(Element::new(1), 123);
    /// assert!(res.is_ok());
    ///
    /// let res = tree.insert(Element::new(1), 123);
    /// assert!(res.is_err());
    /// ```
    ///
    /// Since this function recalculates all hashes after each insert, it can be quite slow. If you
    /// need to insert many elements at the same time, use [`Tree::insert_batch`]
    ///
    /// # Errors
    ///
    /// This function checks for collisions before inserting. That is, if you try to insert `e`,
    /// and the tree contains an element with the same `DEPTH - 1` least significant bits as `e`,
    /// the insert will fail, and return `Err()`.
    ///
    /// Attempting to insert [`Element::NULL_HASH`] will return an error, since this value is used
    /// as a special "empty" value, and can't be inserted into the tree.
    ///
    /// ```rust
    /// # use smirk::*;
    /// let mut tree = Tree::<64, i32>::new();
    /// tree.insert(Element::new(1), 123).unwrap();
    ///
    /// assert_eq!(tree.get(Element::new(1)), Some(&123));
    /// assert_eq!(tree.get(Element::new(2)), None);
    ///
    /// let colliding_element = Element::new(1) + (Element::new(1) << 100);
    /// let error = tree.insert(colliding_element, 123).unwrap_err();
    /// let collision = &error.collisions()[0];
    ///
    /// assert_eq!(collision.in_tree(), Element::new(1));
    /// assert_eq!(collision.inserted(), colliding_element);
    ///
    /// let res = tree.insert(Element::new(1), 123);
    /// assert!(res.is_err());
    /// ```
    pub fn insert(&mut self, element: Element, value: V) -> Result<(), CollisionError>
    where
        C: HashCache,
    {
        self.insert_batch(batch! { element => value })
    }

    /// Insert multiple non-null elements, returning [`Path`]s which prove each element's existence
    /// in the tree at the point where it was inserted
    ///
    /// ```rust
    /// # use smirk::*;
    /// let mut tree = Tree::<64, i32>::new();
    ///
    /// let elements = (1..=10).map(|i| (Element::new(i), i as i32));
    ///
    /// let paths = tree.insert_with_paths(elements).unwrap();
    ///
    /// // each path proves that an element exists
    /// assert!(paths[0].proves(Element::new(1)));
    /// assert!(paths[1].proves(Element::new(2)));
    /// // ...
    ///
    /// // each path links the old root hash to the new root hash
    /// assert_eq!(
    ///     paths[0].compute_root_hash(Element::new(1)),
    ///     paths[1].compute_root_hash(Element::NULL_HASH),
    /// );
    /// assert_eq!(
    ///     paths[1].compute_root_hash(Element::new(2)),
    ///     paths[2].compute_root_hash(Element::NULL_HASH),
    /// );
    /// // ...
    /// ```
    ///
    /// If a collision occurs, the corresponding [`CollisionError`] is returned, but all inserts up
    /// to this point will succeed. If this is unacceptable, consider cloning the tree first.
    pub fn insert_with_paths<I: IntoIterator<Item = (Element, V)>>(
        &mut self,
        entries: I,
    ) -> Result<Vec<Path<DEPTH>>, CollisionError>
    where
        C: HashCache,
    {
        let elements = entries.into_iter();
        let ((_, Some(hint)) | (hint, None)) = elements.size_hint();
        let mut result = Vec::with_capacity(hint);

        for (element, value) in elements {
            // we can't use raw_insert because we need hashes to be recalculated after each insert,
            // otherwise the tree will have stale hashes
            self.insert(element, value)?;
            let path = self.path_for(element);
            result.push(path);
        }

        Ok(result)
    }

    /// Insert multiple non-null elements into the tree, associating them with the default value,
    /// returning [`Path`]s which prove each element's existence in the tree at the point where it
    /// was inserted
    ///
    /// This function is a convenience wrapper around [`Tree::insert_with_paths`]
    pub fn insert_with_paths_default<I>(
        &mut self,
        elements: I,
    ) -> Result<Vec<Path<DEPTH>>, CollisionError>
    where
        I: IntoIterator<Item = Element>,
        V: Default,
        C: HashCache,
    {
        self.insert_with_paths(elements.into_iter().map(|e| (e, V::default())))
    }

    /// Get the value associated with a particular [`Element`], or `None` if the tree doesn't contain
    /// the [`Element`]
    #[inline]
    #[must_use]
    pub fn get(&self, element: Element) -> Option<&V> {
        self.entries.get(&element)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use test_strategy::proptest;

    use crate::{smirk, tree::error::StructName, Collision};

    use super::*;

    #[test]
    fn simple_insert_example() {
        let mut tree = Tree::<64, i32>::new();

        let res = tree.insert(Element::new(3), 3);
        assert!(res.is_ok());
        assert_eq!(
            format!("0x{:x}", tree.root_hash()),
            "0x26debce8a5ba1d092589121944bfc2cc55d858bcd7a697ec2fd1b832b4b20c40"
        );

        let res = tree.insert(Element::new(1), 1);
        assert!(res.is_ok());

        let res = tree.insert(Element::new(1), 1);
        assert!(res.is_err());

        let res = tree.insert(Element::new(2), 2);
        assert!(res.is_ok());

        let colliding_element = Element::new(1) + (Element::new(1) << 100);
        assert!(Element::new(1).collides_with::<64>(colliding_element));

        let error = tree.insert(colliding_element, 123).unwrap_err();
        let collision = &error.collisions()[0];
        assert_eq!(
            collision,
            &Collision {
                in_tree: Element::new(1),
                inserted: colliding_element,
                depth: 64,
                struct_name: StructName::Tree
            }
        );
    }

    #[proptest]
    #[ignore = "slow"]
    fn insert_changes_root_hash(mut tree: Tree<64, i32>, entries: HashMap<Element, i32>) {
        let mut seen_hashes = HashSet::<Element>::from_iter([tree.root_hash()]);
        let mut last_hash = tree.root_hash();

        for (element, value) in entries {
            match tree.insert(element, value) {
                Err(_) => assert_eq!(tree.root_hash(), last_hash),
                Ok(_) => {
                    let new_root_hash = tree.root_hash();
                    assert!(!seen_hashes.contains(&new_root_hash));

                    seen_hashes.insert(new_root_hash);
                    last_hash = new_root_hash;
                }
            }
        }
    }

    #[test]
    fn simple_insert_with_paths_example() {
        let mut tree = Tree::<64, ()>::new();
        let elements = [3, 6, 8].map(Element::new);

        let tree_1: Tree<64, _> = smirk! { 3  };
        let tree_2: Tree<64, _> = smirk! { 3, 6 };
        let tree_3: Tree<64, _> = smirk! { 3, 6, 8 };

        let paths = tree.insert_with_paths_default(elements).unwrap();
        let [first, second, third] = &paths[..] else { panic!() };

        assert_eq!(first.actual_root_hash(), tree_1.root_hash(),);
        assert_eq!(second.actual_root_hash(), tree_2.root_hash(),);
        assert_eq!(third.actual_root_hash(), tree_3.root_hash(),);

        assert!(first.proves(Element::new(3)));
        assert!(second.proves(Element::new(6)));
        assert!(third.proves(Element::new(8)));
    }

    #[test]
    fn insert_with_paths_handles_collisions_and_duplicates() {
        let duplicate = Element::ONE;
        let colliding = Element::ONE + (Element::ONE << 100);

        let mut tree: Tree<64, _> = smirk! { 1 };
        let error = tree.insert_with_paths_default([colliding]).unwrap_err();
        let collision = &error.collisions()[0];

        assert_eq!(collision.in_tree(), Element::ONE);
        assert_eq!(collision.inserted(), colliding);

        let error = tree.insert_with_paths_default([duplicate]).unwrap_err();
        let collision = &error.collisions()[0];
        assert_eq!(collision.inserted(), duplicate);
    }
}
