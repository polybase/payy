use crate::{NodeClientHttp, error::Result};
use client_http::serde_to_query_params;
use element::Element;
use node_interface::MerklePathResponse;
use serde::Serialize;

const MERKLE_PATH: &str = "/merkle";

#[derive(Serialize)]
struct MerkleQuery {
    commitments: String,
}

impl NodeClientHttp {
    /// Fetch Merkle inclusion paths for the provided commitments.
    pub async fn merkle_paths(&self, commitments: &[Element]) -> Result<MerklePathResponse> {
        let query = MerkleQuery {
            commitments: commitments
                .iter()
                .map(|element| element.to_hex())
                .collect::<Vec<_>>()
                .join(","),
        };

        self.http_client
            .get(MERKLE_PATH)
            .query(serde_to_query_params(&query))
            .exec()
            .await?
            .to_value()
            .await
    }
}
