pub mod batch;
mod circuit;
#[allow(clippy::module_inception)]
mod insert;

// Main circuit, batches multiple inserts
pub use batch::*;

// Individual insert
pub use insert::*;
