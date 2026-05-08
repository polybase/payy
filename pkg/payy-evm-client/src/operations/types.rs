use payy_evm_client_interface::{
    ClaimLink, ClaimResolvedInputs, DirectSendDelivery, IncomingNote, IncomingTransfer,
    OwnedNoteState, ParsedClaimLink, PrivacyAccount, ResolvedInputNote,
};

use super::params::{BurnParams, DirectSendParams, EphemeralSendParams, MintParams};
use crate::client::PrivacyNamespace;

/// Claim namespace.
pub struct ClaimClient {
    pub(super) client: PrivacyNamespace,
    pub(super) account: Option<PrivacyAccount>,
}

/// Send namespace.
pub struct SendClient {
    pub(super) client: PrivacyNamespace,
}

/// Generic operation builder.
pub struct OperationBuilder<TPayload> {
    pub(super) client: PrivacyNamespace,
    pub(super) params: OperationParams,
    pub(super) checkpoint: Option<OwnedNoteState>,
    pub(super) resolved_inputs: Vec<ResolvedInputNote>,
    pub(super) claim_inputs: Option<ClaimResolvedInputs>,
    pub(super) _payload: std::marker::PhantomData<TPayload>,
}

#[derive(Clone)]
pub(super) enum OperationParams {
    Mint(MintParams),
    Burn(BurnParams),
    DirectSend(DirectSendParams),
    EphemeralSend(EphemeralSendParams),
    ClaimNote {
        incoming_note: IncomingNote,
        account: Option<PrivacyAccount>,
    },
    ClaimEphemeral {
        incoming_transfer: IncomingTransfer,
        account: Option<PrivacyAccount>,
    },
    ClaimLink {
        link: ClaimLink,
        account: Option<PrivacyAccount>,
    },
}

impl PrivacyNamespace {
    /// Build a mint operation.
    #[must_use]
    pub fn mint(&self, params: MintParams) -> OperationBuilder<()> {
        OperationBuilder::new(self.clone(), OperationParams::Mint(params))
    }

    /// Build a burn operation.
    #[must_use]
    pub fn burn(&self, params: BurnParams) -> OperationBuilder<()> {
        OperationBuilder::new(self.clone(), OperationParams::Burn(params))
    }

    /// Send namespace.
    #[must_use]
    pub fn send(&self) -> SendClient {
        SendClient {
            client: self.clone(),
        }
    }

    /// Claim namespace.
    #[must_use]
    pub fn claim(&self) -> ClaimClient {
        ClaimClient {
            client: self.clone(),
            account: None,
        }
    }
}

impl SendClient {
    /// Build direct send.
    #[must_use]
    pub fn to(&self, params: DirectSendParams) -> OperationBuilder<DirectSendDelivery> {
        OperationBuilder::new(self.client.clone(), OperationParams::DirectSend(params))
    }

    /// Build bearer-style ephemeral send.
    #[must_use]
    pub fn ephemeral(&self, params: EphemeralSendParams) -> OperationBuilder<IncomingTransfer> {
        OperationBuilder::new(self.client.clone(), OperationParams::EphemeralSend(params))
    }
}

impl ClaimClient {
    /// Select a claim output account.
    #[must_use]
    pub fn account(mut self, account: PrivacyAccount) -> Self {
        self.account = Some(account);
        self
    }

    /// Build discovered-note claim.
    #[must_use]
    pub fn note(self, incoming_note: IncomingNote) -> OperationBuilder<IncomingNote> {
        OperationBuilder::new(
            self.client,
            OperationParams::ClaimNote {
                incoming_note,
                account: self.account,
            },
        )
    }

    /// Build bearer-style claim.
    #[must_use]
    pub fn ephemeral(
        self,
        incoming_transfer: IncomingTransfer,
    ) -> OperationBuilder<IncomingTransfer> {
        OperationBuilder::new(
            self.client,
            OperationParams::ClaimEphemeral {
                incoming_transfer,
                account: self.account,
            },
        )
    }

    /// Build claim from link.
    #[must_use]
    pub fn link(self, link: ClaimLink) -> OperationBuilder<ParsedClaimLink> {
        OperationBuilder::new(
            self.client,
            OperationParams::ClaimLink {
                link,
                account: self.account,
            },
        )
    }
}

impl<TPayload> OperationBuilder<TPayload> {
    pub(super) fn new(client: PrivacyNamespace, params: OperationParams) -> Self {
        Self {
            client,
            params,
            checkpoint: None,
            resolved_inputs: Vec::new(),
            claim_inputs: None,
            _payload: std::marker::PhantomData,
        }
    }

    /// Provide an owned-note checkpoint for prepare.
    #[must_use]
    pub fn with_checkpoint(mut self, checkpoint: OwnedNoteState) -> Self {
        self.checkpoint = Some(checkpoint);
        self
    }

    /// Provide a canonical owned input override.
    #[must_use]
    pub fn with_owned_input(mut self, input: ResolvedInputNote) -> Self {
        self.resolved_inputs = vec![input];
        self
    }

    /// Provide canonical claim input overrides.
    #[must_use]
    pub fn with_claim_inputs(mut self, inputs: ClaimResolvedInputs) -> Self {
        self.claim_inputs = Some(inputs);
        self
    }
}
