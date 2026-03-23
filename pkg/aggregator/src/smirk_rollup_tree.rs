use aggregator_interface::{BlockProverError, RollupTree};
use element::Element;
use prover::smirk_metadata::SmirkMetadata;
use smirk::{Batch, CollisionError, Tree, hash_cache::SimpleHashCache};

const MERKLE_TREE_DEPTH: usize = 161;

type InnerTree = Tree<{ MERKLE_TREE_DEPTH }, SmirkMetadata, SimpleHashCache>;

/// [`RollupTree`] implementation backed by a [`smirk::Tree`].
#[derive(Debug, Clone)]
pub struct SmirkRollupTree {
    tree: InnerTree,
    height: u64,
}

impl Default for SmirkRollupTree {
    fn default() -> Self {
        Self {
            tree: Tree::new(),
            height: 0,
        }
    }
}

impl SmirkRollupTree {
    /// Create a new, empty tree with height 0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct the wrapper from an existing [`smirk::Tree`].
    #[must_use]
    pub fn from_tree(tree: InnerTree, height: u64) -> Self {
        Self { tree, height }
    }

    /// Consume the wrapper and return the inner tree.
    #[must_use]
    pub fn into_inner(self) -> (InnerTree, u64) {
        (self.tree, self.height)
    }

    fn map_collision(err: CollisionError) -> BlockProverError {
        BlockProverError::ImplementationSpecific(Box::new(err))
    }
}

impl RollupTree for SmirkRollupTree {
    fn root_hash(&self) -> Element {
        self.tree.root_hash()
    }

    fn height(&self) -> u64 {
        self.height
    }

    fn set_height(&mut self, height: u64) {
        self.height = height;
    }

    fn sibling_path(&self, element: Element) -> Result<Vec<Element>, BlockProverError> {
        Ok(self
            .tree
            .path_for(element)
            .siblings_deepest_first()
            .to_vec())
    }

    fn remove(&mut self, element: Element) -> Result<(), BlockProverError> {
        self.tree.remove(element).map_err(Self::map_collision)
    }

    fn insert(&mut self, entries: &[(Element, u64)]) -> Result<(), BlockProverError> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut batch = Batch::<MERKLE_TREE_DEPTH, SmirkMetadata>::with_capacity(entries.len());
        for (element, inserted_in) in entries {
            batch
                .insert(*element, SmirkMetadata::inserted_in(*inserted_in))
                .map_err(Self::map_collision)?;
        }

        self.tree
            .insert_batch(batch, |_| {}, |_| {})
            .map_err(Self::map_collision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(value: u64) -> Element {
        Element::new(value)
    }

    #[test]
    fn insert_and_remove_round_trip() {
        let mut tree = SmirkRollupTree::new();
        let start_root = tree.root_hash();
        let element = element(42);

        tree.insert(&[(element, 1)]).unwrap();
        assert_ne!(tree.root_hash(), start_root);
        assert_eq!(
            tree.sibling_path(element).unwrap().len(),
            MERKLE_TREE_DEPTH - 1
        );

        tree.remove(element).unwrap();
        assert_eq!(tree.root_hash(), start_root);
    }
}
