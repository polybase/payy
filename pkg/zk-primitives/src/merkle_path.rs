use element::Base;
use element::Element;
use serde::{Deserialize, Serialize};

/// The siblings of a merkle path, for a `smirk::Tree` of depth `DEPTH`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerklePath<const DEPTH: usize> {
    /// The siblings that form the merkle path
    pub siblings: Vec<Element>,
}

impl<const DEPTH: usize> Default for MerklePath<DEPTH> {
    fn default() -> Self {
        let siblings = (1..DEPTH).map(|_| Element::ZERO).collect::<Vec<_>>();

        assert_eq!(siblings.len(), DEPTH - 1);

        Self { siblings }
    }
}

impl<const DEPTH: usize> MerklePath<DEPTH> {
    /// Create a new merkle path from a list of siblings
    ///
    /// # Panics
    ///
    /// If the number of siblings is not equal to `DEPTH - 1`
    #[must_use]
    pub fn new(siblings: Vec<Element>) -> Self {
        assert_eq!(DEPTH - 1, siblings.len(), "Merkle path invalid size");
        MerklePath { siblings }
    }

    /// Compute the root hash of a tree with the given hash at this path (i.e. the state
    /// after the leaf is inserted)
    #[must_use]
    pub fn compute_before_root(&self, hash: Element) -> Element {
        hash::compute_merkle_root(hash, hash, &self.siblings)
    }

    /// Compute the root hash of a tree given siblings, assuming leaf is null (i.e. the state
    /// before the leaf is inserted)
    #[must_use]
    pub fn compute_after_root(&self, hash: Element) -> Element {
        hash::compute_null_root(hash, &self.siblings)
    }

    /// Create a new merkle path, after the insertion of the leaf, leaf must match
    /// the siblings
    #[must_use]
    pub fn apply_leaf(&self, leaf: Element) -> MerklePath<DEPTH> {
        MerklePath {
            siblings: hash::compute_merkle_path_for_leaf(leaf, &self.siblings),
        }
    }
}

impl<const DEPTH: usize> From<[Base; DEPTH]> for MerklePath<DEPTH> {
    fn from(elements: [Base; DEPTH]) -> Self {
        MerklePath::new(
            elements[..DEPTH - 1]
                .iter()
                .copied()
                .map(Element::from_base)
                .collect(),
        )
    }
}

// TODO: this probably won't work! Maybe we should just use a complete path incl root,
// then take off root if needed
// TODO_NOIR
impl<const DEPTH: usize> From<MerklePath<DEPTH>> for [Base; DEPTH] {
    fn from(path: MerklePath<DEPTH>) -> Self {
        path.siblings
            .iter()
            .map(Element::to_base)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}
