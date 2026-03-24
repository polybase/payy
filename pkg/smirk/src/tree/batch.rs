use std::collections::HashSet;

use element::Element;

use crate::{Batch, Collision, CollisionError, Tree, hash_cache::HashCache};

impl<const DEPTH: usize, V, C: HashCache> Tree<DEPTH, V, C> {
    /// Check whether this batch contains any [`Element`]s which would collide with an [`Element`]
    /// that is already in the tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// # use element::Element;
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
    /// [`Element`]: element::Element
    pub fn check_collisions(&self, batch: &Batch<DEPTH, V>) -> Result<(), CollisionError> {
        let mut error = CollisionError::new();

        let tree_lsbs = self
            .entries
            .keys()
            .map(|element| (element, element.lsb(DEPTH - 1)))
            .collect::<Vec<_>>();

        let batch_insert_lsbs = batch
            .insert_elements()
            .map(|e| e.lsb(DEPTH - 1))
            .collect::<HashSet<_>>();
        let batch_remove_lsbs = batch
            .remove_elements()
            .map(|e| (e, e.lsb(DEPTH - 1)))
            .collect::<HashSet<_>>();

        for (tree_element, tree_lsb) in &tree_lsbs {
            if batch_insert_lsbs.contains(tree_lsb) {
                // unwrap fine because there is definitely a collision here
                let batch_element = batch
                    .insert_elements()
                    .chain(batch.remove_elements())
                    .find(|e| e.lsb(DEPTH - 1) == *tree_lsb)
                    .unwrap();

                error.push(Collision {
                    depth: DEPTH,
                    in_tree: **tree_element,
                    inserted: batch_element,
                    struct_name: super::StructName::Tree,
                });
            }
        }

        for (batch_remove_element, batch_remove_element_lsb) in batch_remove_lsbs {
            if !tree_lsbs
                .iter()
                .any(|(_, lsb)| batch_remove_element_lsb == *lsb)
            {
                if option_env!("TEMP_NOIR") != Some("1") {
                    todo!(
                        "we should return something else than collision error here. This is an error that happens if the user wants to remove an element that's not in the tree"
                    );
                }

                error.push(Collision {
                    depth: DEPTH,
                    in_tree: Element::ZERO,
                    inserted: batch_remove_element,
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
    /// # use element::Element;
    /// let mut tree: Tree<64, ()> = smirk! { 1, 2, 3 };
    /// let batch: Batch<64, ()> = batch! { 4, 5 };
    ///
    /// tree.insert_batch(batch, |_| {}, |_| {}).unwrap();
    ///
    /// assert_eq!(tree, smirk! { 1, 2, 3, 4, 5 });
    /// ```
    #[tracing::instrument(skip_all, fields(batch_count = batch.entries.len()))]
    pub fn insert_batch(
        &mut self,
        batch: Batch<DEPTH, V>,
        hash_remove_callback: impl Fn((&Element, &Element)) + Send + Sync,
        hash_set_callback: impl Fn((&Element, &Element, &Element)) + Send + Sync,
    ) -> Result<(), CollisionError> {
        self.check_collisions(&batch)?;

        let Batch {
            entries,
            remove_entries,
            lsbs: _,
        } = batch;

        self.insert_without_hashing(entries, &remove_entries)
            .unwrap();

        tracing::info_span!("recalculate_hashes").in_scope(|| {
            self.tree
                .recalculate_hashes(&self.cache, &hash_remove_callback, &hash_set_callback);
        });

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
        let elements: HashSet<_> = batch.insert_elements().collect();
        let mut tree = Tree::<64, ()>::new();
        tree.insert_batch(batch, |_| {}, |_| {}).unwrap();

        for element in elements {
            assert!(tree.contains_element(&element));
        }
    }

    #[proptest]
    fn fixed_batch_can_always_insert(mut batch: Batch<64, ()>, mut tree: Tree<64, ()>) {
        for (element, ()) in tree.elements() {
            batch.remove(*element).unwrap();
        }

        let elements_in_batch: HashSet<_> = batch.insert_elements().collect();

        tree.insert_batch(batch, |_| {}, |_| {}).unwrap();

        for element in elements_in_batch {
            assert!(tree.contains_element(&element));
        }
    }
}
