mod incoming;
mod owned;

use element::Element;
use payy_evm_client_interface::{
    OwnedNoteState, PrivacyAccount, PrivacyAddress, Result, ValidationErrorKind,
};
use zk_primitives::first_nonce_hash;

use crate::client::PrivacyNamespace;
use crate::util::element_to_b256;

pub use incoming::{IncomingClient, IncomingListParams, IncomingWatchResult};
pub use owned::{BalancesClient, NotesClient, OwnedNoteGetParams};

impl PrivacyNamespace {
    /// Return signer-controlled privacy accounts.
    pub fn accounts(&self) -> Result<Vec<PrivacyAccount>> {
        self.inner.privacy_signer()?.accounts()
    }

    /// Return the default signer-controlled privacy account.
    pub fn default_account(&self) -> Result<Option<PrivacyAccount>> {
        Ok(self.accounts()?.into_iter().next())
    }

    /// Seed the process-local checkpoint cache.
    pub fn set_checkpoint(&self, checkpoint: OwnedNoteState) -> Result<()> {
        validate_checkpoint_shape(&checkpoint)?;
        let key = (checkpoint.privacy_account, checkpoint.token);
        self.inner
            .checkpoints
            .lock()
            .map_err(|_| payy_evm_client_interface::Error::MissingCapability {
                capability: "checkpoint_cache",
            })?
            .insert(key, checkpoint);
        Ok(())
    }

    /// Owned-note state lookup namespace.
    #[must_use]
    pub fn notes(&self) -> NotesClient {
        NotesClient {
            client: self.clone(),
            checkpoint: None,
        }
    }

    /// Private balance namespace.
    #[must_use]
    pub fn balances(&self) -> BalancesClient {
        BalancesClient {
            client: self.clone(),
            checkpoint: None,
        }
    }

    /// Incoming-note discovery namespace.
    #[must_use]
    pub fn incoming(&self) -> IncomingClient {
        IncomingClient {
            client: self.clone(),
        }
    }

    pub(crate) fn resolve_privacy_account(
        &self,
        account: PrivacyAccount,
    ) -> Result<(PrivacyAddress, ResolvedPrivacyAccount)> {
        let address = account.privacy_address();
        match account {
            PrivacyAccount::Signer(signer_account) => Ok((
                address,
                ResolvedPrivacyAccount(PrivacyAccount::Signer(signer_account)),
            )),
            PrivacyAccount::Address(_) => {
                let signer = self.inner.privacy_signer()?;
                Ok((
                    address,
                    ResolvedPrivacyAccount(PrivacyAccount::Signer(
                        payy_evm_client_interface::PrivacySignerAccount {
                            privacy_address: address,
                            signer,
                        },
                    )),
                ))
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct ResolvedPrivacyAccount(PrivacyAccount);

impl ResolvedPrivacyAccount {
    pub(crate) fn signer(&self) -> std::sync::Arc<dyn payy_evm_client_interface::PrivacySigner> {
        match &self.0 {
            PrivacyAccount::Signer(account) => account.signer.clone(),
            PrivacyAccount::Address(_) => unreachable!("resolved accounts are signer-backed"),
        }
    }
}

impl From<ResolvedPrivacyAccount> for PrivacyAccount {
    fn from(account: ResolvedPrivacyAccount) -> Self {
        account.0
    }
}

pub(super) fn ensure_checkpoint_matches(
    checkpoint: &OwnedNoteState,
    privacy_address: PrivacyAddress,
    token: payy_evm_client_interface::Address,
) -> Result<()> {
    if checkpoint.privacy_account != privacy_address || checkpoint.token != token {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::CheckpointMismatch,
        });
    }
    Ok(())
}

pub(super) fn validate_checkpoint_shape(checkpoint: &OwnedNoteState) -> Result<()> {
    if let Some(note) = &checkpoint.owned_note {
        if note
            .source_block
            .is_some_and(|source_block| source_block > checkpoint.checked_block)
        {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::CheckpointMismatch,
            });
        }
        if element_to_b256(note.note.commitment()) != note.commitment {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::CommitmentMismatch,
            });
        }
        if element_to_b256(note.note.nullifier()) != note.nullifier {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::NullifierMismatch,
            });
        }
        if note.note.nonce == Element::ZERO
            && element_to_b256(first_nonce_hash(
                note.note.kind,
                note.note.token,
                note.note.owner,
            )) != note.nonce_hash
        {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::NonceHashMismatch,
            });
        }
    }
    Ok(())
}
