use element::Base;

use crate::circuits::proc_macro_interface::{FromFields, PublicInputs, ToFields};

use super::{Proof, ProofDecodeError};

#[derive(Debug)]
struct TestPublicInputs {
    first: Base,
}

impl ToFields for TestPublicInputs {
    fn to_fields(&self, out: &mut Vec<Base>) {
        out.push(self.first);
    }
}

impl FromFields for TestPublicInputs {
    const FIELD_COUNT: usize = 1;

    fn from_fields(iter: &mut impl Iterator<Item = Base>) -> Self {
        Self {
            first: iter.next().unwrap_or_default(),
        }
    }
}

impl PublicInputs for TestPublicInputs {
    const KEY: &'static [u8] = b"test";
    const ORACLE_HASH_KECCAK: bool = false;
}

#[test]
fn malformed_proof_bytes_return_decode_error() {
    let short = Proof::<TestPublicInputs>::try_from_raw_proof_bytes(vec![0u8; 31]);
    let unaligned = Proof::<TestPublicInputs>::try_from_raw_proof_bytes(vec![0u8; 33]);

    assert!(matches!(
        short,
        Err(ProofDecodeError::PublicInputsTooShort {
            actual: 31,
            required: 32,
        })
    ));
    assert!(matches!(
        unaligned,
        Err(ProofDecodeError::LengthNotMultipleOfField { actual: 33 })
    ));
}
