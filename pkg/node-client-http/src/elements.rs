use crate::{NodeClientHttp, error::Result};
use client_http::HttpBody;
use element::Element;
use node_interface::{ElementsResponse, ListElementsBody};

impl NodeClientHttp {
    /// Returns a list of elements that are in the tree, and when they were
    /// inserted into the tree. If an element is not in the tree, it will be skipped
    /// from the returned elements list
    pub async fn elements(
        &self,
        elements: &[Element],
        include_spent: bool,
    ) -> Result<ElementsResponse> {
        let body = ListElementsBody {
            elements: elements.to_vec(),
            include_spent,
        };

        self.http_client
            .post("/elements", Some(HttpBody::json(body)))
            .exec()
            .await?
            .to_value()
            .await
    }
}
