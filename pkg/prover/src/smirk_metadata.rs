use borsh::{BorshDeserialize, BorshSerialize};

/// The block number of the block that a particular [`Element`] was inserted into a [`Tree`]
///
/// [`Element`]: element::Element
/// [`Tree`]: smirk::Tree
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
#[non_exhaustive]
pub struct SmirkMetadata {
    pub inserted_in: u64,
}

impl SmirkMetadata {
    pub fn inserted_in(inserted_in: u64) -> Self {
        Self { inserted_in }
    }
}
