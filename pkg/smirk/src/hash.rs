use std::sync::OnceLock;

use element::Element;

const COMPUTE_DEPTH: usize = 257;

/// The hash of an empty tree with a given depth
///
/// This function can be defined recursively:
///  - `empty_tree_hash(1) = Element::NULL_HASH`
///  - `empty_tree_hash(n) = hash_merge(empty_tree_hash(n - 1), empty_tree_hash(n - 1))`
///
/// This function computes and caches the first 256 elements, so calls are essentially free (after
/// the initial setup is completed).
///
/// When called with values greater than 256, it will fall back to a naive algorithm, which
/// involves calculating hashes, so is much slower.
///
/// # Panics
///
/// Panics if `depth` is 0, since there is no such thing as a tree with depth 0.
#[inline]
#[must_use]
pub fn empty_tree_hash(depth: usize) -> Element {
    assert_ne!(depth, 0, "the smallest possible tree has depth 1");

    let cache = get_cache();

    cache
        .get(depth - 1)
        .copied()
        .unwrap_or_else(|| fallback(depth))
}

fn fallback(depth: usize) -> Element {
    match depth {
        1..=COMPUTE_DEPTH => get_cache()[depth - 1],
        other => {
            // if you hit this warning, consider increasing `COMPUTE_DEPTH` above
            eprintln!("WARNING - using slow fallback for `empty_tree_hash` for depth: {other}");
            let hashed = fallback(other - 1);
            hash::hash_merge([hashed, hashed])
        }
    }
}

fn get_cache() -> &'static [Element] {
    static CACHE: OnceLock<Vec<Element>> = OnceLock::new();

    CACHE.get_or_init(|| {
        let mut vec = Vec::with_capacity(COMPUTE_DEPTH);
        vec.push(Element::NULL_HASH);

        for _ in 1..COMPUTE_DEPTH {
            let hash = *vec.last().unwrap();
            let new_hash = hash::hash_merge([hash, hash]);
            vec.push(new_hash);
        }

        vec
    })
}
