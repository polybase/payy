use crate::Note;
use element::Element;

/// Burn is a struct that contains the data required to burn a note
///
/// This is used to burn notes in the zk-rollup
#[derive(Clone, Debug)]
pub struct Burn<const L: usize> {
    /// Secret key for the address, required to spend a note
    pub secret_key: Element,
    /// The notes to burn
    pub notes: [Note; L],
    /// The EVM address to send the burnt notes to
    pub to_address: Element,
}

// https://github.com/rust-lang/rust/issues/61415
impl<const L: usize> Default for Burn<L> {
    fn default() -> Self {
        Self {
            secret_key: Element::default(),
            notes: core::array::from_fn(|_| Note::default()),
            to_address: Element::default(),
        }
    }
}

// 2025-05-06T12:10:29.022792Z  INFO node::node::proposal: Committing transaction hash="0x1d527d5fe86464963c5b9439a49e21f5acfd082993df856d9ef898e7919c2b0b" kind=Burn mint_burn_hash=Some(b35547856eecc13f73cb61acc76836b991b91e10372c8dc6e3733bcba92e359) value=1 messages=["0x3", "0x1", "0x989680", "0xb35547856eecc13f73cb61acc76836b991b91e10372c8dc6e3733bcba92e359", "0x8a147ec8b96015e30f732d41f6366b2d72bcff534aae9cb5785b54f5d9f4dc5", "0x0"] input_leaves=["0x239977083fe42eea41ca1dd9831ce022d35ae848a797de1be48a0ef325abb877", "0x0"] output_leaves=["0x0", "0x0"]

// 2025-05-06T12:31:30.465683Z  INFO node::node::proposal: Committing transaction hash="0x2eaaab98a5725b4fe40de234e32eaf079f2206fabedbe54cdae30be74c65c885" kind=Burn mint_burn_hash=Some(2358b58d7d1ed86d3dcb0a409040f20a6daebdca831a4d23c5986e3ae327d07f) value=1 messages=["0x3", "0x1", "0x989680", "0x2358b58d7d1ed86d3dcb0a409040f20a6daebdca831a4d23c5986e3ae327d07f", "0xfbe2f48855f751c01087e52eb4717d1d195bc48c", "0x0"] input_leaves=["0x149f0e7c916ab6863416a1549aa587c7500b3cf3ca220e56e21d96ff64901a63", "0x0"] output_leaves=["0x0", "0x0"]
