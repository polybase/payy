use element::Element;
use ethnum::U256;
use payy_evm_client_interface::{Address, B256, ValidationErrorKind};
use rand::RngCore;
use sha3::{Digest, Keccak256};

pub(crate) fn keccak32(parts: &[&[u8]]) -> B256 {
    let mut hasher = Keccak256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

pub(crate) fn encode_address_word(address: Address) -> B256 {
    let mut word = [0u8; 32];
    word[12..].copy_from_slice(&address);
    word
}

pub(crate) fn b256_to_element(value: B256) -> Element {
    Element::from_be_bytes(value)
}

pub(crate) fn element_to_b256(value: Element) -> B256 {
    value.to_be_bytes()
}

pub(crate) fn address_to_element(address: Address) -> Element {
    b256_to_element(encode_address_word(address))
}

pub(crate) fn random_field() -> Element {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let value = U256::from_be_bytes(bytes) % Element::MODULUS.to_u256();
    Element::from(value)
}

pub(crate) fn random_nonzero_field() -> Element {
    loop {
        let value = random_field();
        if !value.is_zero() {
            return value;
        }
    }
}

pub(crate) fn random_u240_field() -> Element {
    loop {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes[0] = 0;
        bytes[1] = 0;
        let value = Element::from_be_bytes(bytes);
        if !value.is_zero() {
            return value;
        }
    }
}

pub(crate) fn non_zero_address(address: Address) -> bool {
    address.iter().any(|byte| *byte != 0)
}

pub(crate) fn ensure_amount_non_zero(amount: Element) -> payy_evm_client_interface::Result<()> {
    if amount.is_zero() {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::AmountZero,
        });
    }
    Ok(())
}
