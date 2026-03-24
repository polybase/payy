use crate::{Error, Result};

/// Converts a slice to a 32-byte array
pub(crate) fn to_array_32(slice: &[u8]) -> Result<[u8; 32]> {
    if slice.len() != 32 {
        return Err(Error::InvalidKeyLength(32, slice.len()));
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(slice);
    Ok(array)
}
