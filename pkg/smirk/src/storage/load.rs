use core::fmt::Debug;
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use element::Element;
use rocksdb::{DB, IteratorMode};
use wire_message::WireMessage;

use crate::{
    Batch, Tree,
    hash_cache::{KnownHash, SimpleHashCache},
    storage::format::{KeyV2, ValueFormat},
};

use super::{
    Error,
    format::{KeyFormat, ValueV2},
};

#[tracing::instrument(skip_all)]
pub(super) fn load_tree<const DEPTH: usize, V>(
    db: &DB,
) -> Result<Tree<DEPTH, V, SimpleHashCache>, Error>
where
    V: BorshDeserialize + BorshSerialize + Debug + Clone + Send + Sync + 'static,
{
    let mut known_hashes = Vec::new();
    let mut smirk_kv = Vec::new();

    for entry in entries::<V>(db) {
        match entry {
            Ok(RocksbEntry::KnownHash(hash)) => known_hashes.push(hash),
            Ok(RocksbEntry::SmirkKV { key, value }) => smirk_kv.push((key, value)),
            Err(err) => return Err(err),
        }
    }

    let mut cache = SimpleHashCache::new();

    cache.provide_known_hashes(known_hashes);

    let mut smirk = Tree::<DEPTH, V, SimpleHashCache>::new_with_cache(cache);

    let mut batch = Batch::new();
    for (key, value) in smirk_kv {
        batch.insert(key, value)?;
    }

    smirk.insert_batch(batch, |_| {}, |_| {})?;

    Ok(smirk)
}

pub(crate) fn entries<V>(db: &DB) -> impl Iterator<Item = Result<RocksbEntry<V>, Error>> + '_
where
    V: Debug + Clone + Sync + Send + 'static + BorshSerialize + BorshDeserialize,
{
    db.iterator(IteratorMode::Start)
        .filter_map(Result::ok)
        .map(|(key, value)| {
            let key_format = KeyFormat::from_bytes(&key)?;
            let value_format = ValueFormat::from_bytes(&value)?;

            match (key_format, value_format) {
                // either a V1 entry or a V2 smirk-entry KV entry
                (
                    KeyFormat::V1(key) | KeyFormat::V2(KeyV2::Element(key)),
                    ValueFormat::V1(metadata) | ValueFormat::V2(ValueV2::Metadata(metadata)),
                ) => {
                    // refcount should be 0 here
                    let metadata = Arc::try_unwrap(metadata).unwrap();

                    Ok(RocksbEntry::SmirkKV { key, value: metadata })}

                ,
                // a V2 known hash entry
                (
                    KeyFormat::V2(KeyV2::KnownHash { left, right }),
                    ValueFormat::V2(ValueV2::KnownHash(result)),
                ) => Ok(RocksbEntry::KnownHash(KnownHash {
                    left,
                    right,
                    result,
                })),
                // Any other case shouldn't be possible
                _ => Err(Error::DatabaseConsistency),
            }
        })
}

/// Possible meanings of a key-value pair in rocksdb
pub(crate) enum RocksbEntry<V> {
    /// A smirk key-value pair (i.e. an element and its metadata)
    SmirkKV { key: Element, value: V },
    /// A precomputed hash merge
    KnownHash(KnownHash),
}
