// lint-long-file-override allow-max-lines=400
use element::Element;
use sha3::{Digest, Sha3_256};
use web3::types::H160;

/// Implement Serialize and Deserialize for an array of elements of a given size
#[macro_export]
macro_rules! impl_serde_for_element_array {
    ($name:ident, $size:expr) => {
        use ::serde::ser::SerializeSeq;

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let mut seq = serializer.serialize_seq(Some($size))?;
                for element in &self.0 {
                    seq.serialize_element(element)?;
                }
                seq.end()
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::de::Deserializer<'de>,
            {
                struct ArrayVisitor;

                impl<'de> ::serde::de::Visitor<'de> for ArrayVisitor {
                    type Value = $name;

                    fn expecting(
                        &self,
                        formatter: &mut ::std::fmt::Formatter,
                    ) -> ::std::fmt::Result {
                        write!(formatter, "a sequence of {} elements", $size)
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: ::serde::de::SeqAccess<'de>,
                    {
                        let mut elements = [element::Element::default(); $size];
                        for i in 0..$size {
                            elements[i] = seq
                                .next_element()?
                                .ok_or_else(|| ::serde::de::Error::invalid_length(i, &self))?;
                        }
                        Ok($name(elements))
                    }
                }

                deserializer.deserialize_seq(ArrayVisitor)
            }
        }
    };
}

/// Converts a slice of bytes into a vector of Elements by splitting the bytes into 32-byte chunks.
/// Each chunk is converted to an Element using Element::from_be_bytes.
///
/// # Arguments
///
/// * `bytes` - A slice of bytes to be converted into Elements.
///
/// # Returns
///
/// A vector of Elements, where each Element is created from a 32-byte chunk of the input.
///
/// # Panics
///
/// Panics if the length of `bytes` is not a multiple of 32.
#[must_use]
#[inline]
pub fn bytes_to_elements(bytes: &[u8]) -> Vec<Element> {
    assert!(
        bytes.len().is_multiple_of(32),
        "Input bytes length must be a multiple of 32"
    );

    bytes
        .chunks_exact(32)
        .map(|chunk| {
            let chunk_array: [u8; 32] = chunk.try_into().unwrap();
            Element::from_be_bytes(chunk_array)
        })
        .collect()
}

/// Hashes a private key using SHA3-256 and returns the resulting Element.
///
/// # Arguments
///
/// * `private_key` - The private key to be hashed.
///
/// # Returns
///
/// The hashed private key as an Element.
#[must_use]
pub fn hash_private_key_for_psi(private_key: Element) -> Element {
    let mut hasher = Sha3_256::new();
    hasher.update(private_key.to_be_bytes());
    let result = hasher.finalize();

    Element::from_be_bytes(result.into())
}

/// Generates a note kind element from address, chain ID, and note kind format.
/// The format is big endian: <note_kind_format:u16><chain:u160><address:H160><padding:2 bytes>
/// Returns an Element where bytes 31-32 contain the note kind format, bytes 29-30 contain the chain, and bytes 9-28 contain the address.
///
/// # Arguments
///
/// * `note_kind_format` - The note kind format as u8 (will be stored in 2 bytes)
/// * `chain` - The chain ID as u64 (8 bytes)
/// * `address` - The H160 address (20 bytes)
///
/// # Returns
///
/// An Element constructed from the big-endian byte representation.
#[must_use]
pub fn generate_note_kind_bridge_evm(chain: u64, address: H160) -> Element {
    let mut bytes = [0u8; 32];

    // Big endian format: note_kind_format in bytes 0-2, chain in bytes 2-10, address in bytes 10-30
    bytes[0..2].copy_from_slice(&(2u16).to_be_bytes());
    bytes[2..10].copy_from_slice(&chain.to_be_bytes());
    bytes[10..30].copy_from_slice(address.as_bytes());

    Element::from_be_bytes(bytes)
}

/// Extracts the chain identifier from a note kind element.
///
/// The bridge note kind uses the format `<note_kind_format:u16><chain:u160><address:H160><padding:2 bytes>`.
/// This helper copies the chain bytes (index 2 through 9) and converts them to a `u64`.
#[must_use]
pub fn extract_chain_id_from_note_kind(note_kind: Element) -> u64 {
    let note_bytes = note_kind.to_be_bytes();
    let mut chain_bytes = [0u8; 8];
    chain_bytes.copy_from_slice(&note_bytes[2..10]);
    u64::from_be_bytes(chain_bytes)
}

/// Generates a note kind element for USDC on Polygon network.
/// Uses the standard bridged asset format for USDC token on Polygon chain.
///
/// # Returns
///
/// An Element representing the note kind for USDC on Polygon with:
/// - note_kind_format: 2 (ETH based bridged asset)
/// - chain: 137 (Polygon)
/// - address: 0x3c499c542cef5e3811e1192ce70d8cc03d5c3359 (USDC contract address)
#[must_use]
pub fn bridged_polygon_usdc_note_kind() -> Element {
    let chain = 137u64; // Polygon chain
    let address =
        H160::from_slice(&hex::decode("3c499c542cef5e3811e1192ce70d8cc03d5c3359").unwrap());

    generate_note_kind_bridge_evm(chain, address)
}

/// Generates a note kind element for USTB on Ethereum mainnet.
/// Uses the standard bridged asset format for the USTB token on Ethereum.
///
/// # Returns
///
/// An Element representing the note kind for USTB on Ethereum with:
/// - note_kind_format: 2 (ETH based bridged asset)
/// - chain: 1 (Ethereum Mainnet)
/// - address: 0x43415eB6ff9DB7E26A15b704e7A3eDCe97d31C4e (USTB contract address)
#[must_use]
pub fn bridged_ethereum_ustb_note_kind() -> Element {
    let chain = 1u64; // Ethereum mainnet
    let address =
        H160::from_slice(&hex::decode("43415eb6ff9db7e26a15b704e7a3edce97d31c4e").unwrap());

    generate_note_kind_bridge_evm(chain, address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_note_kind() {
        // Test with known values
        let chain = 0x1234u64;
        let address = H160::from([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14,
        ]);

        let result = generate_note_kind_bridge_evm(chain, address);

        // Verify the structure by extracting bytes
        let result_bytes = result.to_be_bytes();

        // Check note_kind_format is in bytes 0-2
        assert_eq!(&result_bytes[0..2], &(2u16).to_be_bytes());

        // Check chain is in bytes 2-10
        assert_eq!(&result_bytes[2..10], &chain.to_be_bytes());

        // Check address is in bytes 10-30
        assert_eq!(&result_bytes[10..30], address.as_bytes());

        // Check last 2 bytes are zero (padding)
        assert_eq!(&result_bytes[30..32], &[0u8; 2]);
    }

    #[test]
    fn test_generate_note_kind_zero_values() {
        let chain = 0x0000u64;
        let address = H160::zero();

        let result = generate_note_kind_bridge_evm(chain, address);
        let result_bytes = result.to_be_bytes();

        // Check note_kind_format is in bytes 0-2
        assert_eq!(&result_bytes[0..2], &(2u16).to_be_bytes());

        // Check chain is in bytes 2-10 (all zeros)
        assert_eq!(&result_bytes[2..10], &[0u8; 8]);

        // Check address is in bytes 10-30 (all zeros)
        assert_eq!(&result_bytes[10..30], &[0u8; 20]);

        // Check last 2 bytes are zero (padding)
        assert_eq!(&result_bytes[30..32], &[0u8; 2]);
    }

    #[test]
    fn test_generate_note_kind_one_values() {
        let chain = 0x0001u64;
        let address = H160::from([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ]);

        let result = generate_note_kind_bridge_evm(chain, address);
        let result_bytes = result.to_be_bytes();

        // Check note_kind_format is in bytes 0-2
        assert_eq!(&result_bytes[0..2], &(2u16).to_be_bytes());

        // Check chain is in bytes 2-10
        assert_eq!(&result_bytes[2..10], &chain.to_be_bytes());

        // Check address is in bytes 10-30
        assert_eq!(&result_bytes[10..30], address.as_bytes());

        // Check last 2 bytes are zero (padding)
        assert_eq!(&result_bytes[30..32], &[0u8; 2]);
    }

    #[test]
    fn test_generate_note_kind_max_values() {
        let chain = 0xFFFF_FFFF_FFFF_FFFF_u64;
        let address = H160::from([0xFF; 20]);

        let result = generate_note_kind_bridge_evm(chain, address);
        let result_bytes = result.to_be_bytes();

        // Check note_kind_format is in bytes 0-2
        assert_eq!(&result_bytes[0..2], &(2u16).to_be_bytes());

        // Check chain bytes are 0xFF
        assert_eq!(&result_bytes[2..10], &[0xFF; 8]);

        // Check address bytes are 0xFF
        assert_eq!(&result_bytes[10..30], &[0xFF; 20]);

        // Check last 2 bytes are zero (padding)
        assert_eq!(&result_bytes[30..32], &[0u8; 2]);
    }

    #[test]
    fn test_generate_note_kind_byte_order() {
        // Test that chain bytes are stored in big endian
        let chain = 0x0123_4567_89AB_CDEF_u64;
        let address = H160::zero();

        let result = generate_note_kind_bridge_evm(chain, address);
        let result_bytes = result.to_be_bytes();

        // In big endian, chain should match the to_be_bytes output
        let expected_chain_bytes = chain.to_be_bytes();
        assert_eq!(&result_bytes[2..10], &expected_chain_bytes);
    }

    #[test]
    fn test_bridged_polygon_usdc_note_kind() {
        let result = bridged_polygon_usdc_note_kind();
        let result_bytes = result.to_be_bytes();

        println!("{:?}", result.to_hex());

        // Check note_kind_format is in bytes 0-2
        assert_eq!(&result_bytes[0..2], &(2u16).to_be_bytes());

        // Check chain is in bytes 2-10 (big endian)
        // 137 (Polygon) = 0x89, so as u64 big endian = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x89]
        let expected_chain_bytes = 137u64.to_be_bytes();
        assert_eq!(&result_bytes[2..10], &expected_chain_bytes);

        // Check address is in bytes 10-30
        let expected_address_bytes =
            hex::decode("3c499c542cef5e3811e1192ce70d8cc03d5c3359").unwrap();
        assert_eq!(&result_bytes[10..30], &expected_address_bytes[..]);

        // Check last 2 bytes are zero (padding)
        assert_eq!(&result_bytes[30..32], &[0u8; 2]);
    }

    #[test]
    fn test_bridged_ethereum_ustb_note_kind() {
        let result = bridged_ethereum_ustb_note_kind();

        // Snapshot of expected hex encoding for USTB on Ethereum.
        let hex_value = format!("0x{}", result.to_hex());
        assert_eq!(
            hex_value,
            "0x0002000000000000000143415eb6ff9db7e26a15b704e7a3edce97d31c4e0000"
        );

        let result_bytes = result.to_be_bytes();

        // Check chain is Ethereum mainnet (1)
        let expected_chain_bytes = 1u64.to_be_bytes();
        assert_eq!(&result_bytes[2..10], &expected_chain_bytes);

        // Check address is encoded correctly
        let expected_address_bytes =
            hex::decode("43415eb6ff9db7e26a15b704e7a3edce97d31c4e").unwrap();
        assert_eq!(&result_bytes[10..30], &expected_address_bytes[..]);

        // Check padding
        assert_eq!(&result_bytes[30..32], &[0u8; 2]);
    }

    #[test]
    fn test_extract_chain_id_from_note_kind() {
        let chain = 137u64;
        let address =
            H160::from_slice(&hex::decode("3c499c542cef5e3811e1192ce70d8cc03d5c3359").unwrap());
        let note_kind = generate_note_kind_bridge_evm(chain, address);

        assert_eq!(extract_chain_id_from_note_kind(note_kind), chain);
    }

    #[test]
    fn test_extract_chain_id_from_note_kind_zero_chain() {
        let note_kind = generate_note_kind_bridge_evm(0, H160::zero());

        assert_eq!(extract_chain_id_from_note_kind(note_kind), 0);
    }

    #[test]
    fn test_extract_chain_id_from_note_kind_max_chain() {
        let chain = u64::MAX;
        let note_kind = generate_note_kind_bridge_evm(chain, H160::zero());

        assert_eq!(extract_chain_id_from_note_kind(note_kind), chain);
    }
}
