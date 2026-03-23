use element::Base;
use noirc_abi::input_parser::InputValue;
use std::collections::BTreeMap;
use zk_primitives::{InputNote, Note};

#[derive(Debug, Clone)]
pub struct BInputNote {
    pub note: BNote,
    pub secret_key: Base,
}

impl From<&InputNote> for BInputNote {
    fn from(note: &InputNote) -> Self {
        BInputNote {
            note: BNote::from(&note.note),
            secret_key: note.secret_key.to_base(),
        }
    }
}

impl From<BInputNote> for InputValue {
    fn from(note: BInputNote) -> Self {
        InputValue::Struct(BTreeMap::from([
            ("note".to_owned(), note.note.into()),
            ("secret_key".to_owned(), InputValue::Field(note.secret_key)),
        ]))
    }
}
#[derive(Debug, Clone)]
pub struct BNote {
    pub kind: Base,
    pub value: Base,
    pub address: Base,
    pub psi: Base,
}

impl From<&Note> for BNote {
    fn from(note: &Note) -> Self {
        BNote {
            kind: note.contract.to_base(),
            value: note.value.to_base(),
            address: note.address.to_base(),
            psi: note.psi.to_base(),
        }
    }
}

impl From<BNote> for InputValue {
    fn from(note: BNote) -> Self {
        let mut struct_ = BTreeMap::new();

        struct_.insert("kind".to_owned(), InputValue::Field(note.kind));
        struct_.insert("value".to_owned(), InputValue::Field(note.value));
        struct_.insert("address".to_owned(), InputValue::Field(note.address));
        struct_.insert("psi".to_owned(), InputValue::Field(note.psi));

        InputValue::Struct(struct_)
    }
}
