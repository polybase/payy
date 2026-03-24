use super::generated::submodules::common::Inputnote as CommonInputNote;
use super::generated::submodules::common::Note as CommonNote;

impl From<zk_primitives::Note> for CommonNote {
    fn from(note: zk_primitives::Note) -> Self {
        CommonNote {
            kind: note.contract,
            value: note.value,
            address: note.address,
            psi: note.psi,
        }
    }
}

impl From<zk_primitives::InputNote> for CommonInputNote {
    fn from(note: zk_primitives::InputNote) -> Self {
        CommonInputNote {
            note: note.note.into(),
            secret_key: note.secret_key,
        }
    }
}

impl From<&zk_primitives::InputNote> for CommonInputNote {
    fn from(note: &zk_primitives::InputNote) -> Self {
        CommonInputNote {
            note: (&note.note).into(),
            secret_key: note.secret_key,
        }
    }
}

impl From<&zk_primitives::Note> for CommonNote {
    fn from(note: &zk_primitives::Note) -> Self {
        CommonNote {
            kind: note.contract,
            value: note.value,
            address: note.address,
            psi: note.psi,
        }
    }
}
