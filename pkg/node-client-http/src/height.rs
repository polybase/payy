use crate::{NodeClientHttp, error::Result};
use node_interface::HeightResponse;

impl NodeClientHttp {
    /// Returns the current height of the chain and the root hash
    pub async fn height(&self) -> Result<HeightResponse> {
        self.http_client
            .get("/height")
            .exec()
            .await?
            .to_value()
            .await
    }
}
