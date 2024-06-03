use crate::{hash_cache::HashCache, Batch, Collision, CollisionError, Tree};

impl<const DEPTH: usize, V, C: HashCache> Tree<DEPTH, V, C> {
    /// Check whether this batch contains any [`Element`]s which would collide with an [`Element`]
    /// that is already in the tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// let tree: Tree<64, ()> = smirk! { 1, 2, 3 };
    ///
    /// let good_batch: Batch<64, ()> = batch! { 4, 5 };
    /// let bad_batch: Batch<64, ()> = batch! { 3, 6 };
    ///
    /// assert!(tree.check_collisions(&good_batch).is_ok());
    ///
    /// let error = tree.check_collisions(&bad_batch).unwrap_err();
    /// assert_eq!(error.collisions().len(), 1);
    /// ```
    ///
    /// [`Element`]: crate::Element
    pub fn check_collisions(&self, batch: &Batch<DEPTH, V>) -> Result<(), CollisionError> {
        let mut error = CollisionError::new();

        let tree_lsbs = self
            .entries
            .keys()
            .map(|element| (element, element.lsb(DEPTH - 1)));

        for (tree_element, tree_lsb) in tree_lsbs {
            if batch.lsbs.contains(&tree_lsb) {
                // unwrap fine because there is definitely a collision here
                let batch_element = batch
                    .elements()
                    .find(|e| e.lsb(DEPTH - 1) == tree_lsb)
                    .unwrap();

                error.push(Collision {
                    depth: DEPTH,
                    in_tree: *tree_element,
                    inserted: batch_element,
                    struct_name: super::StructName::Tree,
                });
            }
        }

        if !error.is_empty() {
            return Err(error);
        }

        Ok(())
    }

    /// Insert a [`Batch`] into the tree
    ///
    /// Note that this is significantly faster than repeated calls to [`Tree::insert`], since it
    /// doesn't need to calculate hashes for each intermediate state
    ///
    /// ```rust
    /// # use smirk::*;
    /// let mut tree: Tree<64, ()> = smirk! { 1, 2, 3 };
    /// let batch: Batch<64, ()> = batch! { 4, 5 };
    ///
    /// tree.insert_batch(batch).unwrap();
    ///
    /// assert_eq!(tree, smirk! { 1, 2, 3, 4, 5 });
    /// ```
    pub fn insert_batch(&mut self, batch: Batch<DEPTH, V>) -> Result<(), CollisionError> {
        self.check_collisions(&batch)?;

        let Batch { entries, .. } = batch;

        for (element, value) in entries {
            // unwrap is fine because we check for collisions earlier
            self.insert_without_hashing(element, value).unwrap();
        }

        self.tree.recalculate_hashes(&self.cache);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use test_strategy::proptest;

    use super::*;

    #[proptest]
    fn can_always_insert_into_empty_tree(batch: Batch<64, ()>) {
        let elements: HashSet<_> = batch.elements().collect();
        let mut tree = Tree::<64, ()>::new();
        tree.insert_batch(batch).unwrap();

        for element in elements {
            assert!(tree.contains_element(element));
        }
    }

    #[proptest]
    fn fixed_batch_can_always_insert(mut batch: Batch<64, ()>, mut tree: Tree<64, ()>) {
        for element in tree.elements() {
            batch.remove(element);
        }

        let elements_in_batch: HashSet<_> = batch.elements().collect();

        tree.insert_batch(batch).unwrap();

        for element in elements_in_batch {
            assert!(tree.contains_element(element));
        }
    }
}
