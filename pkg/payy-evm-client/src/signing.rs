#[cfg(feature = "bb-bindings")]
mod bindings;
#[cfg(all(not(feature = "bb-bindings"), feature = "bb-cli"))]
mod cli;
#[cfg(not(any(feature = "bb-bindings", feature = "bb-cli")))]
mod unavailable;

#[cfg(feature = "bb-bindings")]
pub(crate) use bindings::schnorr_construct_signature;
#[cfg(all(not(feature = "bb-bindings"), feature = "bb-cli"))]
pub(crate) use cli::schnorr_construct_signature;
#[cfg(not(any(feature = "bb-bindings", feature = "bb-cli")))]
pub(crate) use unavailable::schnorr_construct_signature;
