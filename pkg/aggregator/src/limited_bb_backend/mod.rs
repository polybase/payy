use std::sync::Arc;

use aggregator_interface::PrioritizableBbBackend;
use async_trait::async_trait;
use barretenberg_interface::BbBackend;

mod limiter;

#[cfg(test)]
mod tests;

use limiter::ConcurrencyLimiter;

/// Wraps any `BbBackend` and enforces a maximum number of concurrent `prove` calls.
/// Earlier block batches (lower priority values) are given priority access.
#[derive(Clone)]
pub struct LimitedBbBackend {
    inner: Arc<dyn BbBackend>,
    limiter: Arc<ConcurrencyLimiter>,
    priority: u64,
}

impl LimitedBbBackend {
    /// Creates a new backend wrapper that permits at most `max_concurrency` concurrent `prove` calls.
    ///
    /// # Panics
    ///
    /// Panics if `max_concurrency` is zero.
    #[must_use]
    pub fn new(inner: Arc<dyn BbBackend>, max_concurrency: usize) -> Self {
        assert!(
            max_concurrency > 0,
            "max_concurrency must be greater than zero"
        );
        Self {
            inner,
            limiter: Arc::new(ConcurrencyLimiter::new(max_concurrency)),
            priority: u64::MAX,
        }
    }
}

impl PrioritizableBbBackend for LimitedBbBackend {
    fn with_priority(&self, priority: u64) -> Arc<dyn PrioritizableBbBackend> {
        Arc::new(Self {
            inner: Arc::clone(&self.inner),
            limiter: Arc::clone(&self.limiter),
            priority,
        })
    }
}

#[async_trait]
impl BbBackend for LimitedBbBackend {
    async fn prove(
        &self,
        program: &[u8],
        bytecode: &[u8],
        key: &[u8],
        witness: &[u8],
        oracle: bool,
    ) -> barretenberg_interface::error::Result<Vec<u8>> {
        let _permit = self.limiter.acquire(self.priority).await;
        self.inner
            .prove(program, bytecode, key, witness, oracle)
            .await
    }

    async fn verify(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
        key: &[u8],
        oracle: bool,
    ) -> barretenberg_interface::error::Result<()> {
        self.inner.verify(proof, public_inputs, key, oracle).await
    }
}
