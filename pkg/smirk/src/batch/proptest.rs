use std::collections::HashMap;

use element::Element;
use proptest::{arbitrary::StrategyFor, prelude::*, strategy::Map};

use crate::Batch;

impl<const DEPTH: usize, V> Arbitrary for Batch<DEPTH, V>
where
    V: Arbitrary,
{
    type Parameters = ();
    type Strategy = Map<StrategyFor<HashMap<Element, V>>, fn(HashMap<Element, V>) -> Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        any::<HashMap<Element, V>>().prop_map(|map| {
            let mut batch = Batch::with_capacity(map.len());

            for (element, value) in map {
                let _ = batch.insert(element, value);
            }

            batch
        })
    }
}
