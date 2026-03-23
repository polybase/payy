use element::Element;
use serde::{Deserialize, Serialize};

/// Height response for the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightResponse {
    /// Height of the chain
    pub height: u64,
    /// Root hash of the merkle tree
    pub root_hash: Element,
}
