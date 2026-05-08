use crate::Error;

/// Convenience result alias for ramps operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;
