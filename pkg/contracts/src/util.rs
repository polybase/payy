use element::Element;
use ethereum_types::{H160, H256, U256};
use sha3::{Digest, Keccak256};
use web3::ethabi::{Token, encode};

pub fn convert_element_to_h256(element: &Element) -> H256 {
    H256::from_slice(&element.to_be_bytes())
}

pub fn convert_fr_to_u256(element: &Element) -> U256 {
    U256::from_little_endian(&element.to_be_bytes())
}

pub fn convert_web3_secret_key(sk: web3::signing::SecretKey) -> secp256k1::SecretKey {
    secp256k1::SecretKey::from_slice(&sk.secret_bytes()).unwrap()
}

pub fn convert_secp256k1_secret_key(sk: secp256k1::SecretKey) -> web3::signing::SecretKey {
    web3::signing::SecretKey::from_slice(&sk[..]).unwrap()
}

pub fn convert_h160_to_element(h160: &H160) -> Element {
    let mut h256 = [0u8; 32];
    h256[12..32].copy_from_slice(&h160.0);

    Element::from_be_bytes(h256)
}

/// Calculate EIP-712 domain separator locally
///
/// Formula:
/// DomainSeparator := Keccak256(ABIEncode(
///     Keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
///     Keccak256(name),
///     Keccak256(version),
///     chainId,
///     verifyingContract
/// ))
pub fn calculate_domain_separator(
    name: &str,
    version: &str,
    chain_id: U256,
    verifying_contract: ethereum_types::Address,
) -> H256 {
    // EIP712Domain type hash
    let eip712_domain_typehash = Keccak256::digest(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );

    // Hash the name and version strings
    let name_hash = Keccak256::digest(name.as_bytes());
    let version_hash = Keccak256::digest(version.as_bytes());

    // Encode the domain separator according to EIP-712
    let encoded = encode(&[
        Token::FixedBytes(eip712_domain_typehash.to_vec()),
        Token::FixedBytes(name_hash.to_vec()),
        Token::FixedBytes(version_hash.to_vec()),
        Token::Uint(chain_id),
        Token::Address(verifying_contract),
    ]);

    H256::from_slice(&Keccak256::digest(&encoded))
}
