use element::Element;
use payy_evm_client_interface::{B256, Error, Result, ValidationErrorKind};
use zk_primitives::EvmNote;

pub(super) fn validate_received_note(note: EvmNote, commitment: [u8; 32]) -> Result<()> {
    if note.nonce >= Element::MODULUS
        || note.psi >= Element::MODULUS
        || note.owner >= Element::MODULUS
    {
        return Err(Error::Validation {
            kind: ValidationErrorKind::FieldOutOfRange,
        });
    }
    if note.nonce != Element::ZERO {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        });
    }
    let value_bytes = note.value.to_be_bytes();
    if value_bytes[0] != 0 || value_bytes[1] != 0 {
        return Err(Error::Validation {
            kind: ValidationErrorKind::ValueOutOfRange,
        });
    }
    if note.value.is_zero() {
        return Err(Error::Validation {
            kind: ValidationErrorKind::AmountZero,
        });
    }
    if note.commitment().to_be_bytes() != commitment {
        return Err(Error::Validation {
            kind: ValidationErrorKind::CommitmentMismatch,
        });
    }
    Ok(())
}

pub(super) fn append_compact_note(note: EvmNote, out: &mut Vec<u8>) {
    out.push(1);
    let token_bytes = note.token.to_be_bytes();
    out.extend_from_slice(&token_bytes[12..]);
    append_compact_field(note.nonce, out);
    out.extend_from_slice(&note.psi.to_be_bytes());
    out.extend_from_slice(&note.owner.to_be_bytes());
    append_compact_field(note.value, out);
}

pub(super) fn read_compact_note(bytes: &[u8], offset: &mut usize) -> Result<EvmNote> {
    let kind = *bytes.get(*offset).ok_or(Error::Validation {
        kind: ValidationErrorKind::InvalidClaimLink,
    })?;
    *offset += 1;
    if kind != 1 {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        });
    }
    let token = read_token(bytes, offset)?;
    let nonce = read_compact_field(bytes, offset)?;
    let psi = Element::from_be_bytes(read_word_at(bytes, offset)?);
    let owner = Element::from_be_bytes(read_word_at(bytes, offset)?);
    let value = read_compact_field(bytes, offset)?;
    Ok(EvmNote {
        kind: Element::ONE,
        token,
        nonce,
        psi,
        owner,
        value,
    })
}

fn append_compact_field(value: Element, out: &mut Vec<u8>) {
    let bytes = value.to_be_bytes();
    let leading_zeros = bytes.iter().take_while(|byte| **byte == 0).count();
    out.push(u8::try_from(leading_zeros).unwrap_or(32));
    out.extend_from_slice(&bytes[leading_zeros..]);
}

fn read_token(bytes: &[u8], offset: &mut usize) -> Result<Element> {
    let token_bytes = bytes.get(*offset..*offset + 20).ok_or(Error::Validation {
        kind: ValidationErrorKind::InvalidClaimLink,
    })?;
    *offset += 20;
    let mut field = [0u8; 32];
    field[12..].copy_from_slice(token_bytes);
    Ok(Element::from_be_bytes(field))
}

fn read_compact_field(bytes: &[u8], offset: &mut usize) -> Result<Element> {
    let leading_zeros = usize::from(*bytes.get(*offset).ok_or(Error::Validation {
        kind: ValidationErrorKind::InvalidClaimLink,
    })?);
    *offset += 1;
    if leading_zeros > 32 {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        });
    }
    let value_len = 32 - leading_zeros;
    let value = bytes
        .get(*offset..*offset + value_len)
        .ok_or(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        })?;
    *offset += value_len;
    let mut field = [0u8; 32];
    field[leading_zeros..].copy_from_slice(value);
    Ok(Element::from_be_bytes(field))
}

fn read_word_at(bytes: &[u8], offset: &mut usize) -> Result<B256> {
    let word = read_word(bytes, *offset)?;
    *offset += 32;
    Ok(word)
}

pub(super) fn read_word(bytes: &[u8], offset: usize) -> Result<B256> {
    bytes
        .get(offset..offset + 32)
        .and_then(|slice| slice.try_into().ok())
        .ok_or(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        })
}
