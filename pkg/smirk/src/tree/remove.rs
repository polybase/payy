use crate::{Batch, CollisionError, Tree, hash_cache::HashCache};
use element::Element;

impl<const DEPTH: usize, V, C> Tree<DEPTH, V, C> {
    /// Remove a non-null element from the tree
    pub fn remove(&mut self, element: Element) -> Result<(), CollisionError>
    where
        C: HashCache,
    {
        let mut b = Batch::new();
        b.remove(element)?;
        self.insert_batch(b, |_| {}, |_| {})
    }
}
