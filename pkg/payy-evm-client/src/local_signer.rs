use std::sync::Arc;

use element::Element;
use ethnum::U256;
use hash::hash_merge;
use payy_evm_client_interface::{
    B256, EphemeralKeyPair, OwnerSignature, PrivacyAccount, PrivacyAddress, PrivacySigner,
    PrivacySignerAccount, Result, TxnData, ValidationErrorKind,
    grumpkin_public_key_from_private_key, grumpkin_scalar_mul_point,
};
use zk_primitives::EvmNote;

use crate::signing;
use crate::util::{keccak32, random_nonzero_field};

const DERIVATION_DOMAIN: &[u8] = b"payy/grumpkin/v1";
/// Local privacy signer backed by one deterministic private key.
#[derive(Debug, Clone)]
pub struct LocalPrivacySigner {
    private_key: B256,
    privacy_address: PrivacyAddress,
}

impl LocalPrivacySigner {
    /// Build a signer from an already-derived Grumpkin private key.
    pub fn from_grumpkin_private_key(private_key: B256) -> Result<Self> {
        let scalar = Element::from_be_bytes(private_key);
        if scalar.is_zero() || scalar >= Element::MODULUS {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::FieldOutOfRange,
            });
        }
        let (public_key_x, public_key_y) = grumpkin_public_key_from_private_key(private_key)?;
        let privacy_address = PrivacyAddress::from_public_key(public_key_x, public_key_y)?;
        Ok(Self {
            private_key,
            privacy_address,
        })
    }

    /// Build a signer by deriving Grumpkin key material from an EVM key.
    pub fn from_evm_private_key(evm_private_key: B256) -> Result<Self> {
        Self::from_grumpkin_private_key(derive_grumpkin_private_key(evm_private_key))
    }

    /// Return the controlled privacy address.
    #[must_use]
    pub const fn privacy_address(&self) -> PrivacyAddress {
        self.privacy_address
    }

    fn ensure_account(&self, privacy_account: &PrivacyAccount) -> Result<PrivacyAddress> {
        let address = privacy_account.privacy_address();
        if address != self.privacy_address {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::PrivacyAccountMismatch,
            });
        }
        Ok(address)
    }
}

impl PrivacySigner for LocalPrivacySigner {
    fn accounts(&self) -> Result<Vec<PrivacyAccount>> {
        Ok(vec![signer_account(
            Arc::new(self.clone()) as Arc<dyn PrivacySigner>,
            self.privacy_address,
        )])
    }

    fn sign_tx_commitment(
        &self,
        privacy_account: PrivacyAccount,
        tx_commitment: B256,
    ) -> Result<OwnerSignature> {
        self.ensure_account(&privacy_account)?;
        let (public_key_x, public_key_y) = grumpkin_public_key_from_private_key(self.private_key)?;
        let (s, e) = signing::schnorr_construct_signature(tx_commitment, self.private_key)?;
        let mut signature = Vec::with_capacity(64);
        signature.extend_from_slice(&s);
        signature.extend_from_slice(&e);
        Ok(OwnerSignature {
            signature,
            public_key_x,
            public_key_y,
        })
    }

    fn decrypt_sender_note(
        &self,
        privacy_account: PrivacyAccount,
        txn_data: TxnData,
    ) -> Result<Option<EvmNote>> {
        let address = self.ensure_account(&privacy_account)?;
        self.decrypt_note_for_address(
            address,
            txn_data.sender_encrypted_note,
            txn_data.user_encrypted_key,
        )
    }

    fn decrypt_recipient_note(
        &self,
        privacy_account: PrivacyAccount,
        txn_data: TxnData,
    ) -> Result<Option<EvmNote>> {
        let address = self.ensure_account(&privacy_account)?;
        self.decrypt_note_for_address(
            address,
            txn_data.recipient_encrypted_note,
            txn_data.recipient_encrypted_key,
        )
    }

    fn generate_ephemeral_key(&self) -> Result<EphemeralKeyPair> {
        let private_key = random_nonzero_field().to_be_bytes();
        let (public_key_x, public_key_y) = grumpkin_public_key_from_private_key(private_key)?;
        let privacy_address = PrivacyAddress::from_public_key(public_key_x, public_key_y)?;
        Ok(EphemeralKeyPair {
            private_key,
            privacy_address,
        })
    }
}

impl LocalPrivacySigner {
    fn decrypt_note_for_address(
        &self,
        address: PrivacyAddress,
        encrypted_note: [B256; 5],
        encrypted_key: [B256; 4],
    ) -> Result<Option<EvmNote>> {
        let Some(note) = self.decrypt_note(encrypted_note, encrypted_key)? else {
            return Ok(None);
        };
        if note.owner != address.owner()? {
            return Ok(None);
        }
        Ok(Some(note))
    }

    fn decrypt_note(
        &self,
        encrypted_note: [B256; 5],
        encrypted_key: [B256; 4],
    ) -> Result<Option<EvmNote>> {
        if encrypted_key.iter().all(|word| *word == [0; 32]) {
            return Ok(None);
        }
        let ephemeral_x = Element::from_be_bytes(encrypted_key[0]);
        let ephemeral_y = Element::from_be_bytes(encrypted_key[1]);
        let (shared_x, shared_y) =
            grumpkin_scalar_mul_point(self.private_key, ephemeral_x, ephemeral_y)?;
        let symmetric_key = field_sub(
            Element::from_be_bytes(encrypted_key[2]),
            hash_merge([shared_x, shared_y]),
        );
        let plaintext = core::array::from_fn::<_, 5, _>(|index| {
            field_sub(
                Element::from_be_bytes(encrypted_note[index]),
                hash_merge([symmetric_key, Element::new(index as u64)]),
            )
        });
        let note = EvmNote {
            kind: Element::ONE,
            token: plaintext[0],
            nonce: plaintext[1],
            psi: plaintext[2],
            owner: plaintext[3],
            value: plaintext[4],
        };
        Ok(Some(note))
    }
}

/// Build a signer-backed account from a local signer.
#[must_use]
pub fn signer_account(
    signer: Arc<dyn PrivacySigner>,
    privacy_address: PrivacyAddress,
) -> PrivacyAccount {
    PrivacyAccount::Signer(PrivacySignerAccount {
        privacy_address,
        signer,
    })
}

/// Derive the canonical Grumpkin private scalar from an EVM secp256k1 key.
#[must_use]
pub fn derive_grumpkin_private_key(evm_private_key: B256) -> B256 {
    let digest = keccak32(&[DERIVATION_DOMAIN, &evm_private_key]);
    let x = U256::from_be_bytes(digest);
    let scalar = U256::ONE + x % (Element::MODULUS.to_u256() - U256::ONE);
    Element::from(scalar).to_be_bytes()
}

fn field_sub(lhs: Element, rhs: Element) -> Element {
    Element::from_base(lhs.to_base() - rhs.to_base())
}
