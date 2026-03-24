use element::Element;
use serde::Deserialize;

/// Response payload containing Merkle inclusion paths for provided commitments.
#[derive(Debug, Deserialize)]
pub struct MerklePathResponse {
    /// Each entry corresponds to the requested commitment order.
    pub paths: Vec<Vec<Element>>,
}
