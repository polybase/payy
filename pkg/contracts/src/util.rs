use ethereum_types::{H160, H256, U256};
// use smirk::Element;
use zk_primitives::Element;

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
