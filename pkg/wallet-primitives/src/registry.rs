use element::Element;
use encrypt::{
    EncryptedAsymmetricData, EncryptedSymmetricData, PublicKey, StaticSecret, asymmetric_decrypt,
    asymmetric_encrypt, generate_symmetric_key, symmetric_decrypt, symmetric_encrypt,
};
use serde::{Deserialize, Serialize};
use zk_primitives::Note;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Encrypt error: {0}")]
    Encrypt(#[from] encrypt::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryNote {
    pub note: Note,
    pub private_key: Element,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RegistryNoteRef<'a> {
    note: &'a Note,
    private_key: &'a Element,
    memo: Option<&'a String>,
}

pub struct EncryptedRegistryNote {
    pub encrypted_key: Vec<u8>,
    pub encrypted_note: Vec<u8>,
}

impl EncryptedRegistryNote {
    pub fn to_asymmetric_data(&self) -> Result<EncryptedAsymmetricData> {
        EncryptedAsymmetricData::from_bytes(&self.encrypted_key).map_err(Error::Encrypt)
    }

    pub fn to_symmetric_data(&self) -> Result<EncryptedSymmetricData> {
        EncryptedSymmetricData::from_bytes(&self.encrypted_note).map_err(Error::Encrypt)
    }
}

pub fn encode_registry_note(
    to_public_key: Element,
    private_key: Element,
    note: &zk_primitives::Note,
    memo: Option<String>,
) -> Result<EncryptedRegistryNote> {
    // Encrypt the note data
    let registry_note = RegistryNoteRef {
        note,
        private_key: &private_key,
        memo: memo.as_ref(),
    };

    // Convert note to JSON string
    let note_json_str = serde_json::to_string(&registry_note)?;

    // Convert JSON string to bytes
    let note_json_str_bytes = note_json_str.as_bytes();

    // Symmetric key to be asymmetric encrypted
    let ephemeral_pk = generate_symmetric_key();
    let asymmetric_encrypted_symmetric_key =
        asymmetric_encrypt(&PublicKey::from(to_public_key.to_be_bytes()), &ephemeral_pk)?
            .to_bytes();

    // Note json bytes data to be symmetric encrypted
    let encrypted_note = symmetric_encrypt(&ephemeral_pk, note_json_str_bytes)?.to_bytes();

    // Return the encrypted key and note
    Ok(EncryptedRegistryNote {
        encrypted_key: asymmetric_encrypted_symmetric_key,
        encrypted_note,
    })
}

pub fn decode_registry_note(
    private_key: Element,
    encrypted_note: &EncryptedRegistryNote,
) -> Result<RegistryNote> {
    // Convert private key to StaticSecret for asymmetric decryption
    let private_key_bytes = private_key.to_be_bytes();
    let static_secret = StaticSecret::from(private_key_bytes);

    // Convert encrypted key to EncryptedAsymmetricData
    let encrypted_key_data = encrypted_note.to_asymmetric_data()?;

    // Decrypt the symmetric key using the private key
    let symmetric_key_bytes = asymmetric_decrypt(&static_secret, &encrypted_key_data)?;

    // Convert symmetric key bytes to Key
    let symmetric_key = encrypt::Key::from_slice(&symmetric_key_bytes);

    // Convert encrypted note to EncryptedSymmetricData
    let encrypted_note_data = encrypted_note.to_symmetric_data()?;

    // Decrypt the note data using the symmetric key
    let decrypted_note_bytes = symmetric_decrypt(symmetric_key, &encrypted_note_data)?;

    // Convert decrypted bytes to string
    let decrypted_note_str = String::from_utf8(decrypted_note_bytes).map_err(|_| {
        Error::Serialization(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Decrypted data is not valid UTF-8",
        )))
    })?;

    // Deserialize JSON string back to RegistryNote
    let registry_note = serde_json::from_str::<RegistryNote>(&decrypted_note_str)?;

    Ok(registry_note)
}
