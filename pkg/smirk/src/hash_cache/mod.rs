use std::sync::Arc;

use dashmap::{DashMap, mapref::entry::Entry};
use element::Element;
use hash::hash_merge;

pub use self::metrics::CacheMetrics;

mod metrics;

/// A known result of computation [`hash_merge([left, right])`][hash_merge]
///
/// [hash_merge]: hash::hash_merge
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KnownHash {
    /// The left input [`Element`]
    pub left: Element,
    /// The right input [`Element`]
    pub right: Element,
    /// The result of applying [`hash_merge([left, right])`][hash_merge]
    pub result: Element,
}

/// Types which can be used to speed up hash computations (perhaps by storing known values in a
/// table)
///
/// Take special care when implementing this trait, since incorrect external implementations can
/// cause [`Tree`] (and, by extension, [`Persistent`]) to exhibit unspecified
/// behaviour. Note that this is not [Undefined Behaviour][ub] - it is more akin to having
/// mismatched [`PartialEq`] and [`Hash`] implementations
///
/// [`Tree`]: crate::Tree
/// [`Persistent`]: crate::storage::Persistent
/// [`PartialEq`]: std::cmp::PartialEq
/// [`PartialEq`]: std::cmp::PartialEq
/// [`Hash`]: std::hash::Hash
///
/// [ub]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
pub trait HashCache: Sync + 'static {
    /// Calculate [`hash_merge([left, right])`][hash_merge], potentially using data in `self` to
    /// speed up the calculation
    ///
    /// Implementors should make sure that the result from this function *always* matches the
    /// result from [`hash_merge`]
    fn hash(&self, left: Element, right: Element) -> Element {
        hash_merge([left, right])
    }
}

/// A ZST that does no caching - the default cache for [`Tree`]
///
/// [`Tree`]: crate::Tree
#[derive(Debug, Clone, Default)]
pub struct NoopHashCache;

impl HashCache for NoopHashCache {}

/// A simple cache (conceptually an [`Arc<Mutex<HashMap<(Element, Element), Element>>>`])
///
/// It is cheap to clone, thread-safe, but has limited eviction capabilities
#[derive(Debug, Clone, Default)]
pub struct SimpleHashCache {
    inner: Arc<DashMap<(Element, Element), Element>>,
    initial: Arc<Vec<KnownHash>>,
    metrics: metrics::CacheMetrics,
}

impl HashCache for SimpleHashCache {
    #[inline]
    fn hash(&self, left: Element, right: Element) -> Element {
        self.metrics.incr_hashes();

        if let Ok(result) = self
            .initial
            .binary_search_by_key(&(left, right), |hash| (hash.left, hash.right))
        {
            self.metrics.incr_cache_hits();
            return self.initial[result].result;
        }

        match self.inner.entry((left, right)) {
            Entry::Occupied(entry) => {
                self.metrics.incr_cache_hits();
                *entry.get()
            }
            Entry::Vacant(entry) => {
                self.metrics.incr_cache_misses();
                *entry.insert(hash_merge([left, right]))
            }
        }
    }
}

impl SimpleHashCache {
    /// Create a new, empty [`SimpleHashCache`]
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of precomputed hashes in this cache
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether this cache contains no entries
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    /// Provide a set of known hashes to this cache
    ///
    /// Note that these hashes will not be validated - providing incorrect hashes will lead to
    /// incorrect results
    #[inline]
    pub fn provide_known_hashes(&mut self, mut hashes: Vec<KnownHash>) {
        hashes.sort_unstable_by_key(|hash| (hash.left, hash.right));

        assert!(
            self.initial.is_empty(),
            "provide_known_hashes should only be called once"
        );
        self.initial = Arc::new(hashes);
    }

    /// Remove the result of a hash from memory
    #[inline]
    pub fn evict(&self, left: Element, right: Element) {
        self.inner.remove(&(left, right));
    }

    /// Remove all hashes from the cache
    #[inline]
    pub fn evict_all(&self) {
        self.inner.clear();
    }

    /// Get metrics for this cache
    #[inline]
    #[must_use]
    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_cache_persists_hashes() {
        let cache = SimpleHashCache::default();

        cache.hash(Element::new(1), Element::new(2));
        cache.hash(Element::new(3), Element::new(4));

        assert!(
            cache
                .inner
                .contains_key(&(Element::new(1), Element::new(2)))
        );
        assert!(
            cache
                .inner
                .contains_key(&(Element::new(3), Element::new(4)))
        );

        assert_eq!(cache.metrics().hashes(), 2);
        assert_eq!(cache.metrics().cache_hits(), 0);
        assert_eq!(cache.metrics().cache_misses(), 2);

        cache.hash(Element::new(1), Element::new(2));

        assert_eq!(cache.metrics().hashes(), 3);
        assert_eq!(cache.metrics().cache_hits(), 1);
        assert_eq!(cache.metrics().cache_misses(), 2);
    }
}
