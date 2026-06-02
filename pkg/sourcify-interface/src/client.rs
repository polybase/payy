use async_trait::async_trait;

use crate::{ContractField, ContractResponse, Result};

/// Request for a Sourcify v2 contract lookup.
#[derive(Clone, Debug)]
pub struct ContractLookup {
    /// Decimal chain id.
    pub chain_id: u64,
    /// EIP-55 checksum address.
    pub address: String,
    /// Requested Sourcify fields.
    pub fields: Vec<ContractField>,
    /// Maximum decoded response bytes.
    pub response_cap_bytes: usize,
}

/// Sourcify contract lookup interface.
#[async_trait]
pub trait SourcifyClient: Send + Sync + 'static {
    /// Fetch a Sourcify v2 contract record.
    async fn contract(&self, lookup: &ContractLookup) -> Result<ContractResponse>;
}
