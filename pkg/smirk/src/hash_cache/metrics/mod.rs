use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// A container for metrics relating to hashing, useful for debugging
#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    hashes: Arc<AtomicUsize>,
    cache_hits: Arc<AtomicUsize>,
    cache_misses: Arc<AtomicUsize>,
}

impl CacheMetrics {
    /// The number of times the `hash` function has been called on this cache
    #[inline]
    #[must_use]
    pub fn hashes(&self) -> usize {
        self.hashes.load(Ordering::Relaxed)
    }

    /// The number of times the cache has returned a cached value
    #[inline]
    #[must_use]
    pub fn cache_hits(&self) -> usize {
        self.cache_hits.load(Ordering::Relaxed)
    }

    /// The number of times the cache had to compute a new value
    #[inline]
    #[must_use]
    pub fn cache_misses(&self) -> usize {
        self.cache_misses.load(Ordering::Relaxed)
    }

    pub(crate) fn incr_hashes(&self) {
        self.hashes.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn incr_cache_hits(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn incr_cache_misses(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }
}
