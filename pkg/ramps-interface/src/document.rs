use async_trait::async_trait;
use primitives::serde::Base64Bytes;
use serde::{Deserialize, Serialize};
use test_spy::spy_mock;
use uuid::Uuid;

use crate::error::Result;

/// Request payload for uploading an encrypted document.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDocumentRequest {
    pub data: Base64Bytes,
    pub key: String,
}

/// Response payload exposing the document hash identifier.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateDocumentResponse {
    pub hash: String,
}

/// Interface for document management endpoints.
#[spy_mock]
#[async_trait]
pub trait DocumentsInterface: Send + Sync {
    /// Persist a new encrypted document and return its canonical hash.
    async fn create_document(
        &self,
        wallet_id: Uuid,
        request: CreateDocumentRequest,
    ) -> Result<CreateDocumentResponse>;
}
