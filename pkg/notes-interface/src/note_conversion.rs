use element::Element;

use crate::data::{Note, NoteWithPk};

impl<'a> From<&'a Note> for NoteWithPk {
    fn from(note: &'a Note) -> Self {
        Self {
            private_key: note.private_key,
            note: zk_primitives::Note {
                kind: Element::new(2),
                contract: note.kind,
                address: note.address,
                psi: note.psi,
                value: note.value,
            },
        }
    }
}

impl<'a> From<&'a Note> for zk_primitives::Note {
    fn from(note: &'a Note) -> Self {
        Self {
            kind: Element::new(2),
            contract: note.kind,
            address: note.address,
            psi: note.psi,
            value: note.value,
        }
    }
}
