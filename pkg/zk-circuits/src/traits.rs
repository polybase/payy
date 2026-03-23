use std::sync::Arc;

use crate::Result;
use async_trait::async_trait;
use barretenberg_interface::BbBackend;

#[async_trait]
pub trait Prove {
    type Proof: Verify + Send + Sync;

    async fn prove(&self, bb_backend: &dyn BbBackend) -> Result<Self::Proof>;
}

#[async_trait]
pub trait Verify: Send + Sync {
    #[must_use = "verification result must be explicitly handled"]
    async fn verify(&self, bb_backend: &dyn BbBackend) -> Result<()>;
}

#[unimock::unimock(api = CircuitMock)]
#[async_trait]
pub trait Circuit<Input: Sync, Output>: Send + Sync {
    async fn prove(&self, input: &Input, bb_backend: Arc<dyn BbBackend>) -> Result<Output>;
}

pub use crate::circuits::generated::AggAggCircuitInterface;
pub use crate::circuits::generated::AggFinalCircuitInterface;
pub use crate::circuits::generated::AggUtxoCircuitInterface;
