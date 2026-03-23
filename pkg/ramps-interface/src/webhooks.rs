use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use test_spy::spy_mock;

use crate::provider::Provider;

use crate::error::Result;

/// High-level metadata describing the inbound webhook request.
#[derive(Debug, Clone, Default)]
pub struct WebhookContext {
    pub path: String,
    pub headers: HashMap<String, String>,
    pub peer_addr: Option<String>,
    pub provider: Option<Provider>,
}

/// Wrapper around provider webhook payloads with auxiliary request data.
#[derive(Debug, Clone)]
pub struct WebhookRequest {
    pub context: WebhookContext,
    pub raw_body: Vec<u8>,
    pub provider: Provider,
}

/// Interface covering provider webhook entry points.
#[spy_mock]
#[async_trait]
pub trait WebhooksInterface: Send + Sync {
    async fn handle_webhook(&self, request: WebhookRequest) -> Result<Value>;
}
