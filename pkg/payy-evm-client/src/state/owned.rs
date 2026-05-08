// lint-long-file-override allow-max-lines=300
use element::Element;
use payy_evm_client_interface::{
    Address, OwnedNote, OwnedNoteState, PrivacyAccount, PrivacyAddress, PrivateBalance,
    PrivateBalanceState, Result, ValidationErrorKind,
};
use zk_primitives::{first_nonce_hash, next_nonce_hash};

use super::{ResolvedPrivacyAccount, ensure_checkpoint_matches, validate_checkpoint_shape};
use crate::client::{CacheKey, PrivacyNamespace};
use crate::util::{address_to_element, element_to_b256};

/// Owned-note lookup params.
#[derive(Debug, Clone)]
pub struct OwnedNoteGetParams {
    /// Privacy account selector.
    pub privacy_account: PrivacyAccount,
    /// Token.
    pub token: Address,
}

/// Owned-note namespace.
pub struct NotesClient {
    pub(super) client: PrivacyNamespace,
    pub(super) checkpoint: Option<OwnedNoteState>,
}

/// Balance namespace.
pub struct BalancesClient {
    pub(super) client: PrivacyNamespace,
    pub(super) checkpoint: Option<OwnedNoteState>,
}

impl NotesClient {
    /// Provide a caller-owned checkpoint for this lookup.
    #[must_use]
    pub fn with_checkpoint(mut self, checkpoint: OwnedNoteState) -> Self {
        self.checkpoint = Some(checkpoint);
        self
    }

    /// Resolve latest owned-note state.
    pub async fn get(self, params: OwnedNoteGetParams) -> Result<OwnedNoteState> {
        self.client
            .resolve_owned_note(params, self.checkpoint)
            .await
    }
}

impl BalancesClient {
    /// Provide a caller-owned checkpoint for this lookup.
    #[must_use]
    pub fn with_checkpoint(mut self, checkpoint: OwnedNoteState) -> Self {
        self.checkpoint = Some(checkpoint);
        self
    }

    /// Resolve current private balance.
    pub async fn get(self, params: OwnedNoteGetParams) -> Result<PrivateBalanceState> {
        let state = self
            .client
            .resolve_owned_note(params, self.checkpoint)
            .await?;
        let balance = state.owned_note.as_ref().map(|owned_note| PrivateBalance {
            privacy_account: state.privacy_account,
            token: state.token,
            spendable: owned_note.note.value,
        });
        Ok(PrivateBalanceState {
            balance,
            owned_note_state: state,
        })
    }
}

impl PrivacyNamespace {
    pub(crate) async fn resolve_owned_note(
        &self,
        params: OwnedNoteGetParams,
        checkpoint: Option<OwnedNoteState>,
    ) -> Result<OwnedNoteState> {
        let (address, account) = self.resolve_privacy_account(params.privacy_account)?;
        if let Some(checkpoint) = checkpoint {
            ensure_checkpoint_matches(&checkpoint, address, params.token)?;
            return self
                .validate_or_discard_checkpoint(checkpoint, address, params.token, account)
                .await;
        }
        if let Some(cached) = self.cache_get((address, params.token)) {
            return self
                .validate_or_discard_checkpoint(cached, address, params.token, account)
                .await;
        }
        self.full_owned_note_lookup(address, params.token, account)
            .await
    }

    async fn validate_or_discard_checkpoint(
        &self,
        checkpoint: OwnedNoteState,
        privacy_address: PrivacyAddress,
        token: Address,
        account: ResolvedPrivacyAccount,
    ) -> Result<OwnedNoteState> {
        match self
            .validate_and_refresh_checkpoint(checkpoint, account.clone())
            .await
        {
            Ok(state) => Ok(state),
            Err(payy_evm_client_interface::Error::Validation {
                kind:
                    ValidationErrorKind::CommitmentMismatch
                    | ValidationErrorKind::NullifierMismatch
                    | ValidationErrorKind::NonceHashMismatch,
            }) => {
                self.full_owned_note_lookup(privacy_address, token, account)
                    .await
            }
            Err(err) => Err(err),
        }
    }

    async fn full_owned_note_lookup(
        &self,
        privacy_address: PrivacyAddress,
        token: Address,
        account: ResolvedPrivacyAccount,
    ) -> Result<OwnedNoteState> {
        self.inner.validate_read_chain().await?;
        let checked_block = self.inner.read_client.get_block_number().await?;
        let owner = privacy_address.owner()?;
        let token_element = address_to_element(token);
        let nonce_hash = first_nonce_hash(Element::ONE, token_element, owner);
        let note = self
            .scan_nonce_chain(privacy_address, token, nonce_hash, &account)
            .await?;
        let state = OwnedNoteState {
            privacy_account: privacy_address,
            token,
            owned_note: note,
            checked_block,
        };
        self.cache_put(state.clone());
        Ok(state)
    }

    async fn scan_nonce_chain(
        &self,
        privacy_address: PrivacyAddress,
        token: Address,
        mut nonce_hash: Element,
        account: &ResolvedPrivacyAccount,
    ) -> Result<Option<OwnedNote>> {
        loop {
            let Some(note) = self
                .lookup_one_note(privacy_address, token, nonce_hash, account)
                .await?
            else {
                return Ok(None);
            };
            if !self.bridge().element_exists(note.nullifier).await? {
                return Ok(Some(note));
            }
            nonce_hash = next_nonce_hash(
                note.note.kind,
                note.note.token,
                note.note.owner,
                note.note.nonce + Element::ONE,
                note.note.psi,
            );
        }
    }

    async fn lookup_one_note(
        &self,
        privacy_address: PrivacyAddress,
        token: Address,
        nonce_hash: Element,
        account: &ResolvedPrivacyAccount,
    ) -> Result<Option<OwnedNote>> {
        let nonce_hash_b256 = element_to_b256(nonce_hash);
        let Some(txn_hash) = self
            .bridge()
            .get_txn_hash_by_nonce_hash(nonce_hash_b256)
            .await?
        else {
            return Ok(None);
        };
        let txn_data = self.bridge().get_txn_data(txn_hash).await?;
        let Some(note) = account
            .signer()
            .decrypt_sender_note(account.clone().into(), txn_data)?
        else {
            return Ok(None);
        };
        if note.token != address_to_element(token) || note.owner != privacy_address.owner()? {
            return Ok(None);
        }
        Ok(Some(OwnedNote {
            note,
            commitment: element_to_b256(note.commitment()),
            nullifier: element_to_b256(note.nullifier()),
            nonce_hash: nonce_hash_b256,
            source_block: None,
            source_tx_hash: None,
            source_bridge_tx_hash: Some(txn_hash),
        }))
    }

    async fn validate_and_refresh_checkpoint(
        &self,
        checkpoint: OwnedNoteState,
        account: ResolvedPrivacyAccount,
    ) -> Result<OwnedNoteState> {
        validate_checkpoint_shape(&checkpoint)?;
        if let Some(note) = &checkpoint.owned_note {
            let refreshed = self
                .lookup_one_note(
                    checkpoint.privacy_account,
                    checkpoint.token,
                    Element::from_be_bytes(note.nonce_hash),
                    &account,
                )
                .await?;
            if !matches!(
                refreshed.as_ref(),
                Some(owned)
                    if owned.commitment == note.commitment
                        && owned.nullifier == note.nullifier
                        && owned.note == note.note
            ) {
                return self
                    .full_owned_note_lookup(checkpoint.privacy_account, checkpoint.token, account)
                    .await;
            }
            if !self.bridge().element_exists(note.nullifier).await? {
                let mut fresh = checkpoint;
                fresh.checked_block = self.inner.read_client.get_block_number().await?;
                self.cache_put(fresh.clone());
                return Ok(fresh);
            }
            let next_nonce_hash = next_nonce_hash(
                note.note.kind,
                note.note.token,
                note.note.owner,
                note.note.nonce + Element::ONE,
                note.note.psi,
            );
            let checked_block = self.inner.read_client.get_block_number().await?;
            let owned_note = self
                .scan_nonce_chain(
                    checkpoint.privacy_account,
                    checkpoint.token,
                    next_nonce_hash,
                    &account,
                )
                .await?;
            let fresh = OwnedNoteState {
                privacy_account: checkpoint.privacy_account,
                token: checkpoint.token,
                owned_note,
                checked_block,
            };
            self.cache_put(fresh.clone());
            return Ok(fresh);
        }
        self.full_owned_note_lookup(checkpoint.privacy_account, checkpoint.token, account)
            .await
    }

    fn cache_get(&self, key: CacheKey) -> Option<OwnedNoteState> {
        self.inner
            .checkpoints
            .lock()
            .ok()
            .and_then(|cache| cache.get(&key).cloned())
    }

    fn cache_put(&self, state: OwnedNoteState) {
        if let Ok(mut cache) = self.inner.checkpoints.lock() {
            cache.insert((state.privacy_account, state.token), state);
        }
    }
}
