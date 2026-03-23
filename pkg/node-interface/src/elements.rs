use element::Element;
use primitives::block_height::BlockHeight;
use serde::{Deserialize, Serialize};

/// Query for list elemenets
#[derive(Debug, Serialize, Deserialize)]
pub struct ListElementsQuery {
    /// String comma seperated list of elements to lookup
    pub elements: String,
    /// When true, include elements that have been spent (seen historically)
    #[serde(default)]
    pub include_spent: bool,
}

/// Body for listing elements via POST to avoid URL length limitations
#[derive(Debug, Serialize, Deserialize)]
pub struct ListElementsBody {
    /// Elements to lookup in the tree
    pub elements: Vec<Element>,
    /// When true, include elements that have been spent (seen historically)
    #[serde(default)]
    pub include_spent: bool,
}

/// Response from the elements endpoint
pub type ElementsResponse = Vec<ElementsResponseSingle>;

/// Response item from the elements endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementsResponseSingle {
    /// The element being returned
    pub element: Element,
    /// Block height that the element was included in
    pub height: u64,
    /// Root hash of the block the element was included in
    pub root_hash: Element,
    /// Txn hash
    pub txn_hash: Element,
    /// Whether the element has been spent
    pub spent: bool,
}

/// Snapshot of the tree as of a block height
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockTreeResponse {
    /// Block height the snapshot corresponds to
    pub height: BlockHeight,
    /// Root hash for the block at `height`
    pub root_hash: Element,
    /// Optional ordered list of elements present in the tree at `height`
    ///
    /// When requesting a diff, this may be omitted so clients can apply the diff to their cached
    /// tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elements: Option<Vec<Element>>,
    /// Optional diff from a previous height
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<BlockTreeDiffChanges>,
}

/// Snapshot of block tree elements at a block height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTreeSnapshot {
    /// Block height the snapshot corresponds to
    pub height: BlockHeight,
    /// Root hash for the block at `height`
    pub root_hash: Element,
    /// Ordered list of elements present in the tree at `height`
    pub elements: Vec<Element>,
}

/// Diff response including metadata for transforming a previous block's tree into the current tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTreeDiff {
    /// Block height the diff corresponds to
    pub height: BlockHeight,
    /// Root hash for the block at `height`
    pub root_hash: Element,
    /// Detailed diff entries
    pub diff: BlockTreeDiffChanges,
}

/// Detailed diff entries needed to transform a previous block's tree into the current tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTreeDiffChanges {
    /// Height being diffed from
    pub from_height: BlockHeight,
    /// Elements to insert to reach the new tree
    pub additions: Vec<Element>,
    /// Elements to remove to reach the new tree
    pub removals: Vec<Element>,
}
