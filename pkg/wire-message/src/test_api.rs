use borsh::{BorshDeserialize, BorshSerialize};

use crate::strum_macros::EnumCount;
use crate::{Error, WireMessage};

/// Dummy message type for use in testing
#[derive(
    ::borsh::BorshSerialize,
    ::borsh::BorshDeserialize,
    EnumCount, // we don't use the macro here because it's in the same crate
    Debug,
    Clone,
    PartialEq,
)]
pub enum DummyMsg<T = i32> {
    V1(T),
}

impl<T: BorshDeserialize + BorshSerialize + Send + Sync + 'static> WireMessage for DummyMsg<T> {
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        1
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, Error> {
        Err(Self::max_version_error())
    }
}

impl<T> DummyMsg<T> {
    pub fn inner(&self) -> &T {
        match self {
            Self::V1(inner) => inner,
        }
    }
}
