// lint-long-file-override allow-max-lines=300
use element::Element;
use hash::compute_merkle_root;
use payy_evm_client_interface::{
    B256, OwnerSignature, PrivacyAccount, ResolvedInputNote, Result, ValidationErrorKind,
};
use zk_primitives::{EvmNote, field_to_address};

use super::proof::{
    CircuitMerklePath, CircuitOwnerSignature, ClaimInputs, OwnedSpendInput, SpendInput,
};
use super::types::OperationBuilder;
use crate::LocalPrivacySigner;
use crate::local_signer::signer_account;
use crate::util::{b256_to_element, element_to_b256};

impl<TPayload> OperationBuilder<TPayload> {
    pub(super) async fn resolve_owned_spend_input(
        &self,
        privacy_account: PrivacyAccount,
        token: payy_evm_client_interface::Address,
        allow_padding: bool,
    ) -> Result<OwnedSpendInput> {
        if let Some(input) = self.resolved_inputs.first() {
            return self
                .resolved_owned_input_from_override(input, allow_padding)
                .await;
        }
        let state = self
            .resolve_owned_note(privacy_account.clone(), token)
            .await?;
        let Some(owned_note) = state.owned_note else {
            if allow_padding {
                return Ok(OwnedSpendInput::Padding {
                    merkle_path: zero_merkle_path(),
                    recent_root: b256_to_element(self.client.bridge().get_root().await?),
                });
            }
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::MissingOwnedNote,
            });
        };
        self.spend_input_from_note(
            owned_note.note,
            owned_note.commitment,
            owned_note.nullifier,
            None,
            None,
        )
        .await
        .map(|input| OwnedSpendInput::Real(Box::new(input)))
    }

    pub(super) async fn incoming_spend_input(&self, note: EvmNote) -> Result<SpendInput> {
        self.spend_input_from_note(
            note,
            element_to_b256(note.commitment()),
            element_to_b256(note.nullifier()),
            None,
            None,
        )
        .await
    }

    pub(super) async fn claim_inputs(
        &self,
        account: PrivacyAccount,
        incoming_note: EvmNote,
    ) -> Result<ClaimInputs> {
        if let Some(inputs) = &self.claim_inputs {
            let own = self
                .resolved_owned_input_from_override(&inputs.owned_input, true)
                .await?;
            let incoming = self
                .spend_input_from_note(
                    inputs.incoming_input.owned_note.note,
                    inputs.incoming_input.owned_note.commitment,
                    inputs.incoming_input.owned_note.nullifier,
                    Some(inputs.incoming_input.merkle_path.clone()),
                    Some(inputs.incoming_input.recent_root),
                )
                .await?;
            if incoming.note != incoming_note {
                return Err(payy_evm_client_interface::Error::Validation {
                    kind: ValidationErrorKind::CommitmentMismatch,
                });
            }
            return Ok(ClaimInputs { own, incoming });
        }
        let token = field_to_address(incoming_note.token);
        Ok(ClaimInputs {
            own: self.resolve_owned_spend_input(account, token, true).await?,
            incoming: self.incoming_spend_input(incoming_note).await?,
        })
    }

    async fn resolved_owned_input_from_override(
        &self,
        input: &ResolvedInputNote,
        allow_padding: bool,
    ) -> Result<OwnedSpendInput> {
        match input {
            ResolvedInputNote::Real(real) => self
                .spend_input_from_note(
                    real.owned_note.note,
                    real.owned_note.commitment,
                    real.owned_note.nullifier,
                    Some(real.merkle_path.clone()),
                    Some(real.recent_root),
                )
                .await
                .map(|input| OwnedSpendInput::Real(Box::new(input))),
            ResolvedInputNote::Padding(padding) => {
                if !allow_padding {
                    return Err(payy_evm_client_interface::Error::Validation {
                        kind: ValidationErrorKind::MissingOwnedNote,
                    });
                }
                Ok(OwnedSpendInput::Padding {
                    merkle_path: zero_merkle_path(),
                    recent_root: b256_to_element(padding.recent_root),
                })
            }
        }
    }

    async fn spend_input_from_note(
        &self,
        note: EvmNote,
        commitment: B256,
        nullifier: B256,
        merkle_path: Option<Vec<B256>>,
        recent_root: Option<B256>,
    ) -> Result<SpendInput> {
        if element_to_b256(note.commitment()) != commitment {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::CommitmentMismatch,
            });
        }
        if element_to_b256(note.nullifier()) != nullifier {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::NullifierMismatch,
            });
        }
        if self.client.bridge().element_exists(nullifier).await? {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::NoteSpent,
            });
        }
        let (path, resolved_root) = if let Some(path) = merkle_path {
            (path, recent_root)
        } else {
            let resolved = self.client.bridge().get_merkle_path(commitment).await?;
            (resolved.siblings, Some(resolved.root))
        };
        let merkle_path = merkle_path_from_words(path)?;
        let recent_root = b256_to_element(match resolved_root {
            Some(root) => root,
            None => self.client.bridge().get_root().await?,
        });
        let computed_root =
            compute_merkle_root(note.commitment(), note.commitment(), &merkle_path.path);
        if computed_root != recent_root {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::CommitmentMismatch,
            });
        }
        Ok(SpendInput {
            note,
            merkle_path,
            recent_root,
            commitment: note.commitment(),
            nullifier: note.nullifier(),
        })
    }

    pub(super) fn signer_for_account(
        &self,
        account: &PrivacyAccount,
    ) -> Result<std::sync::Arc<dyn payy_evm_client_interface::PrivacySigner>> {
        match account {
            PrivacyAccount::Address(_) => self.client.inner.privacy_signer(),
            PrivacyAccount::Signer(account) => Ok(account.signer.clone()),
        }
    }

    pub(super) fn sign_owner(
        &self,
        account: PrivacyAccount,
        tx_commitment: Element,
    ) -> Result<CircuitOwnerSignature> {
        owner_signature_to_circuit(
            self.signer_for_account(&account)?
                .sign_tx_commitment(account, element_to_b256(tx_commitment))?,
        )
    }

    pub(super) fn sign_ephemeral_owner(
        &self,
        private_key: B256,
        tx_commitment: Element,
    ) -> Result<CircuitOwnerSignature> {
        let signer =
            std::sync::Arc::new(LocalPrivacySigner::from_grumpkin_private_key(private_key)?);
        let address = signer.privacy_address();
        let account = signer_account(signer, address);
        owner_signature_to_circuit(
            self.signer_for_account(&account)?
                .sign_tx_commitment(account, element_to_b256(tx_commitment))?,
        )
    }
}

fn merkle_path_from_words(path: Vec<B256>) -> Result<CircuitMerklePath> {
    if path.len() > 160 {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::MerklePathInvalid,
        });
    }
    let mut siblings = [Element::ZERO; 160];
    for (index, sibling) in path.into_iter().enumerate() {
        siblings[index] = b256_to_element(sibling);
    }
    Ok(CircuitMerklePath { path: siblings })
}

fn zero_merkle_path() -> CircuitMerklePath {
    CircuitMerklePath {
        path: [Element::ZERO; 160],
    }
}

fn owner_signature_to_circuit(signature: OwnerSignature) -> Result<CircuitOwnerSignature> {
    let bytes: [u8; 64] = signature.signature.try_into().map_err(|_| {
        payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::InvalidOwnerSignature,
        }
    })?;
    Ok(CircuitOwnerSignature {
        signature: bytes,
        public_key_x: signature.public_key_x,
        public_key_y: signature.public_key_y,
    })
}
