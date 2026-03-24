use borsh::{BorshDeserialize, BorshSerialize};

/// Height of the block.
pub type BlockHeight = u64;

/// Block height delta that measures the difference between `BlockHeight`s.
pub type BlockHeightDelta = u64;

/// Balance is type for storing amounts of tokens.
pub type Balance = u128;

/// Validator is a public key or identifier of the validator.
#[expect(
    dead_code,
    reason = "Validator type reserved for future doomslug networking APIs"
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, serde::Serialize)]
pub struct Validator(pub [u8; 32]);
