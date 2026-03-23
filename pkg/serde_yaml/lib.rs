//! Local shim crate that re-exports `serde_yaml_ng` under the historic
//! `serde_yaml` crate name so upstream dependencies continue to compile.

pub use serde_yaml_ng::*;
pub use serde_yaml_ng::{mapping, value, with};
