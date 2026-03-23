use element::Element;

/// Get the address from a private key
///
/// # Arguments
///
/// * `private_key` - The private key to get the address from.
///
/// # Returns
///
/// The address derived from the private key.
#[must_use]
#[inline]
pub fn get_address_for_private_key(private_key: Element) -> Element {
    hash::hash_merge([private_key, Element::ZERO])
}
