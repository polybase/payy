use ::proptest::{arbitrary::StrategyFor, prelude::*, strategy::Map};

use crate::{
    Batch, Tree,
    hash_cache::{HashCache, NoopHashCache, SimpleHashCache},
};

impl<const DEPTH: usize, V, C> Arbitrary for Tree<DEPTH, V, C>
where
    V: Arbitrary,
    C: HashCache + Arbitrary,
{
    type Parameters = ();
    type Strategy = Map<StrategyFor<(C, Batch<DEPTH, V>)>, fn((C, Batch<DEPTH, V>)) -> Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        any::<(C, Batch<DEPTH, V>)>().prop_map(|(cache, batch)| {
            let mut tree = Tree::new_with_cache(cache);
            tree.insert_batch(batch, |_| {}, |_| {}).unwrap();
            tree
        })
    }
}

impl Arbitrary for NoopHashCache {
    type Parameters = ();
    type Strategy = Just<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        Just(Self)
    }
}

impl Arbitrary for SimpleHashCache {
    type Parameters = ();
    type Strategy = Just<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        Just(Self::default())
    }
}
