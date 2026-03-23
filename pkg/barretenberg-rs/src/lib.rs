#[cfg(feature = "bb_rs")]
mod binding;

#[cfg(not(feature = "bb_rs"))]
mod mock;

#[cfg(feature = "bb_rs")]
pub use binding::BindingBackend;

#[cfg(not(feature = "bb_rs"))]
pub use mock::BindingBackend;
