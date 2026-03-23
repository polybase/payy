use element::Element;
use sha3::{Digest, Keccak256};

pub mod public_key;
pub mod registry;

pub fn derive_private_key(bytes: &[u8], private_key: Element) -> Element {
    // Convert the private key to big-endian bytes
    let private_key_bytes = private_key.to_be_bytes();

    // Create a combined buffer with DEPOSIT_HEX prefix
    let mut combined = Vec::with_capacity(bytes.len() + private_key_bytes.len());
    combined.extend_from_slice(bytes);
    combined.extend_from_slice(&private_key_bytes);

    // Compute keccak256 hash
    let hash = Keccak256::digest(&combined);

    // Convert hash to Element
    Element::from_be_bytes(hash.as_slice().try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_derive_private_key() {
        let base_pk_el =
            Element::from_str("0x32e9d283c6b6e42c7b57cb7183ce8c43b493bcce17eddc3c3aca3cb003bb075f")
                .unwrap();
        let derived_pk_el =
            Element::from_str("0xcdeb86d2b4aa02dbff2e3bc1356f4292cc6f0327cf4b4db2003b57385d3a5ac1")
                .unwrap();
        assert_eq!(
            derive_private_key(&[222, 96, 81, 112], base_pk_el),
            derived_pk_el
        )
    }
}
