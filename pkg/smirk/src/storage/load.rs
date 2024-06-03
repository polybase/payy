use core::fmt::Debug;
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use rocksdb::{IteratorMode, DB};
use wire_message::WireMessage;
use zk_primitives::Element;

use crate::{
    hash_cache::{KnownHash, SimpleHashCache},
    storage::format::{KeyV2, ValueFormat},
    Batch, Tree,
};

use super::{
    format::{KeyFormat, ValueV2},
    Error,
};

pub(super) fn load_tree<const DEPTH: usize, V>(
    db: &DB,
) -> Result<Tree<DEPTH, V, SimpleHashCache>, Error>
where
    V: BorshDeserialize + BorshSerialize + Debug + Clone + Send + Sync + 'static,
{
    let entries = entries::<V>(db).collect::<Result<Vec<_>, _>>()?;

    let cache = SimpleHashCache::new();

    cache.provide_known_hashes(entries.iter().filter_map(|entry| match entry {
        RocksbEntry::KnownHash(hash) => Some(*hash),
        RocksbEntry::SmirkKV { .. } => None,
    }));

    let kv_pairs = entries.into_iter().filter_map(|entry| match entry {
        RocksbEntry::KnownHash(..) => None,
        RocksbEntry::SmirkKV { key, value } => Some((key, value)),
    });

    let mut smirk = Tree::<DEPTH, V, SimpleHashCache>::new_with_cache(cache);

    let mut batch = Batch::new();
    for (key, value) in kv_pairs {
        batch.insert(key, value)?;
    }

    smirk.insert_batch(batch)?;

    Ok(smirk)
}

fn entries<V>(db: &DB) -> impl Iterator<Item = Result<RocksbEntry<V>, Error>> + '_
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
enum RocksbEntry<V> {
    /// A smirk key-value pair (i.e. an element and its metadata)
    SmirkKV { key: Element, value: V },
    /// A precomputed hash merge
    KnownHash(KnownHash),
}
