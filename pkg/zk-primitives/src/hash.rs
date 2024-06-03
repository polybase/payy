use poseidon_circuit::poseidon::primitives::{ConstantLength, Hash, P128Pow5T3};

use crate::{Base, Element};

#[cfg(feature = "test-api")]
static HASH_COUNTER: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// The number of times [`hash_merge`] has been called
#[cfg(feature = "test-api")]
pub fn hash_count() -> usize {
    HASH_COUNTER.load(core::sync::atomic::Ordering::Relaxed)
}

/// Reset the count returned by [`hash_count`] to 0
#[cfg(feature = "test-api")]
pub fn reset_hash_count() {
    HASH_COUNTER.store(0, core::sync::atomic::Ordering::Relaxed);
}

#[cfg(feature = "test-api")]
static HASH_ELEMENT_COUNTER: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

/// The number of elements that have been hashed together
#[cfg(feature = "test-api")]
pub fn hash_element_count() -> usize {
    HASH_ELEMENT_COUNTER.load(core::sync::atomic::Ordering::Relaxed)
}

/// Reset the count returned by [`hash_element_count`] to 0
#[cfg(feature = "test-api")]
pub fn reset_hash_element_count() {
    HASH_ELEMENT_COUNTER.store(0, core::sync::atomic::Ordering::Relaxed);
}

/// Hash two elements together
///
/// This function is used to calculate the hash of a parent node from the hash of its children,
/// i.e.: `parent_hash = hash_merge(left_hash, right_hash)`
///
/// ```rust
/// # use zk_primitives::*;
/// let a = hash_merge([Element::new(1), Element::new(2)]);
/// let b = hash_merge([Element::new(1), Element::new(3)]);
/// let c = hash_merge([Element::new(2), Element::new(3)]);
///
/// assert_ne!(a, b);
/// assert_ne!(a, c);
/// assert_ne!(b, c);
/// ```
/// This operation is not symmetric:
/// ```rust
/// # use zk_primitives::*;
/// let a = Element::new(1);
/// let b = Element::new(2);
///
/// let ab = hash_merge([a, b]);
/// let ba = hash_merge([b, a]);
///
/// assert_ne!(ab, ba);
/// ```
#[inline]
#[must_use]
pub fn hash_merge<const N: usize>(elements: [Element; N]) -> Element {
    type H<const N: usize> = Hash<Base, P128Pow5T3<Base>, ConstantLength<N>, 3, 2>;

    #[cfg(feature = "test-api")]
    {
        HASH_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        HASH_ELEMENT_COUNTER.fetch_add(N, core::sync::atomic::Ordering::Relaxed);
    }

    let hash = H::<N>::init().hash(elements.map(Element::to_base));
    Element::from_base(hash)
}

/// Hash a slice of bytes
///
/// ```rust
/// # use zk_primitives::*;
/// let hash_1 = hash_bytes(&[1, 2, 3, 4]);
/// let hash_2 = hash_bytes(&[1, 2, 3, 5]);
///
/// assert_ne!(hash_1, hash_2);
/// ```
#[inline]
#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> Element {
    // an element is slightly smaller than a "u254". For convenience, we're just going to pretend
    // it's a u128. If we need the extra perf, we can be a bit more compact here.

    let initial = Element::BYTE_HASH_IV;

    let elements_from_bytes = bytes
        .chunks(core::mem::size_of::<u128>())
        .map(bytes_to_element);

    core::iter::once(initial)
        .chain(elements_from_bytes)
        .reduce(|left, right| hash_merge([left, right]))
        .unwrap() // there's always at least 1 element
}

/// Convert a slice of bytes with length in the range `1..=16` to an [`Element`]
///
/// If there are fewer than 16 bytes, the lower bytes are padded with zeroes
fn bytes_to_element(bytes: &[u8]) -> Element {
    let mut padded_bytes = [0; 16];
    padded_bytes[0..bytes.len()].copy_from_slice(bytes);
    u128::from_be_bytes(padded_bytes).into()
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use rand_chacha::{rand_core::SeedableRng, ChaChaRng};

    use super::*;

    #[derive(serde::Serialize)]
    struct MergeResult {
        left: Element,
        right: Element,
        merged: Element,
    }

    impl MergeResult {
        pub fn new(left: Element, right: Element) -> Self {
            let merged = hash_merge([left, right]);

            Self {
                left,
                right,
                merged,
            }
        }
    }

    #[test]
    fn hash_merge_snapshot_test() {
        let special_cases = [
            MergeResult::new(Element::NULL_HASH, Element::NULL_HASH),
            MergeResult::new(Element::NULL_HASH, Element::ONE),
            MergeResult::new(Element::ONE, Element::NULL_HASH),
        ];

        let mut rng = ChaChaRng::from_seed([0; 32]);
        let random_cases = core::iter::from_fn(|| {
            let left = Element::secure_random(&mut rng);
            let right = Element::secure_random(&mut rng);
            Some(MergeResult::new(left, right))
        });

        let results: Vec<_> = special_cases
            .into_iter()
            .chain(random_cases.take(100))
            .collect();

        insta::assert_json_snapshot!(results);
    }

    #[derive(serde::Serialize)]
    struct ByteResult {
        #[serde(with = "hex::serde")]
        bytes: Vec<u8>,
        hash: Element,
    }

    impl ByteResult {
        fn new(bytes: &[u8]) -> Self {
            let hash = hash_bytes(bytes);
            let bytes = bytes.to_vec();
            Self { bytes, hash }
        }
    }

    #[test]
    fn hash_bytes_snapshot_test() {
        let special_cases = [
            ByteResult::new(&[]),
            ByteResult::new(&[0]),
            ByteResult::new(&[0; 16]),
        ];

        let mut rng = ChaChaRng::from_seed([0; 32]);

        let random_cases = core::iter::from_fn(|| {
            let mut bytes = [0; 64];
            rng.fill(&mut bytes);
            Some(ByteResult::new(&bytes))
        });

        let results: Vec<_> = special_cases
            .into_iter()
            .chain(random_cases.take(100))
            .collect();

        insta::assert_json_snapshot!(results);
    }
}
