use element::Element;
use encrypt::{PublicKey, StaticSecret};

pub fn to_public_key(private_key: Element) -> Element {
    let private_key_bytes = private_key.to_be_bytes();
    let static_secret = StaticSecret::from(private_key_bytes);
    let public_key = PublicKey::from(&static_secret);
    Element::from_be_bytes(*public_key.as_bytes())
}
