use std::fmt::Debug;

use borsh::{BorshDeserialize, BorshSerialize};
use rand_derive2::RandGen;
use serde::{Deserialize, Serialize};

microtype::microtype! {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize, RandGen, Serialize, Deserialize)]
    pub u64 {
        #[derive(Default)]
        #[int]  // add maths traits
        BlockHeight,
    }
}

// transparent debug impl
impl Debug for BlockHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl BlockHeight {
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}
