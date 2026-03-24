use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use element::Element;
use wire_message::{WireMessage, wire_message};

#[derive(Debug, Clone)]
#[wire_message]
pub(super) enum KeyFormat {
    V1(Element),
    V2(KeyV2),
}

impl WireMessage for KeyFormat {
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        match self {
            Self::V1(_) => 1,
            Self::V2(_) => 2,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        match self {
            Self::V1(element) => Ok(Self::V2(KeyV2::Element(element))),
            Self::V2(_) => Err(Self::max_version_error()),
        }
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub(super) enum KeyV2 {
    Element(Element),
    KnownHash { left: Element, right: Element },
}

#[derive(Debug, Clone)]
#[wire_message]
pub(super) enum ValueFormat<T: Clone> {
    V1(Arc<T>),
    V2(ValueV2<T>),
}

impl<T> WireMessage for ValueFormat<T>
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        match self {
            Self::V1(_) => 1,
            Self::V2(_) => 2,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        match self {
            Self::V1(metadata) => Ok(Self::V2(ValueV2::Metadata(metadata))),
            Self::V2(_) => Err(Self::max_version_error()),
        }
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub(super) enum ValueV2<V: Clone> {
    Metadata(Arc<V>),
    KnownHash(Element),
}
