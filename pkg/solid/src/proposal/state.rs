use crate::peer::Peer;
use crate::txn::Txn;
use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};
use std::fmt::Debug;

use super::ProposalManifest;

pub trait ProposalState: Clone + Debug + Send + Sync + 'static {
    type State: Debug
        + Clone
        + Default
        + Send
        + PartialEq
        + Eq
        + BorshSerialize
        + BorshDeserialize
        + 'static;

    fn genesis() -> Self::State {
        Self::State::default()
    }

    // Validate the proposal, called when the proposal is the "current" proposal, i.e.
    // just before an accept is sent. This allows the validate fn to take into account
    // the last confirmed proposal state when validating this proposal.
    fn validate<P: Peer>(&self, _: &ProposalManifest<P, Self>) -> bool {
        true
    }

    /// Hash of the proposal/block
    fn hash<P: Peer>(manifest: &ProposalManifest<P, Self>) -> [u8; 32];
}

impl ProposalState for Vec<Txn> {
    type State = Self;

    fn hash<P: Peer>(manifest: &ProposalManifest<P, Self>) -> [u8; 32] {
        #[allow(clippy::unwrap_used)]
        #[allow(clippy::disallowed_methods)]  // fine since we're just hashing, not deserializing
        let bytes = Sha256::digest(borsh::to_vec(&manifest).unwrap());
        bytes.into()
    }
}
