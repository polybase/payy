use std::sync::Arc;

use payy_evm_client_interface::Result;

use super::ClientInner;
use crate::bridge::BridgeClient;
use crate::links::LinksClient;

/// Privacy operation namespace.
#[derive(Clone)]
pub struct PrivacyNamespace {
    pub(crate) inner: Arc<ClientInner>,
}

impl PrivacyNamespace {
    pub(crate) fn from_inner(inner: &Arc<ClientInner>) -> Self {
        Self {
            inner: inner.clone(),
        }
    }

    /// Raw bridge reads.
    #[must_use]
    pub fn bridge(&self) -> BridgeClient {
        BridgeClient::new(self.inner.clone())
    }

    /// Claim-link parsing helpers.
    #[must_use]
    pub fn links(&self) -> LinksClient {
        LinksClient
    }

    /// Validate the read adapter is connected to the configured network.
    pub async fn validate_read_chain(&self) -> Result<()> {
        self.inner.validate_read_chain().await
    }
}
