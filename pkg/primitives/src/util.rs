//! Common utility functions for use in other packages

use std::borrow::Cow;

/// Truncate the given string in the middle, keeping `max_len/2` characters from
/// the beginning and `max_len/2` characters from the end.
/// If the string is smaller than `max_len`, it is returned as is.
pub fn middle_truncate(s: &str, max_len: usize) -> Cow<str> {
    if s.len() <= max_len {
        Cow::Borrowed(s)
    } else {
        let half = max_len / 2;
        Cow::Owned(format!("{}…{}", &s[..half], &s[s.len() - half..]))
    }
}
