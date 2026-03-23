use crate::{
    AppState, Peer,
    proposal::{Manifest, ProposalAccept, ProposalHash},
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum SolidEvent<P: Peer, S: AppState> {
    /// Send a new proposal to the network, as we are the the next leader
    Propose {
        last_proposal_hash: ProposalHash,
        height: u64,
        skips: u64,
        accepts: Vec<ProposalAccept<P>>,
    },

    /// Proposal has been confirmed and should be committed
    /// to the data store
    Commit {
        /// Proposal manifest to commit
        manifest: Manifest<P, S>,
        /// An unconfirmed proposal that justified the proposal being committed
        confirmed_by: Manifest<P, S>,
    },

    /// Accept a proposal, this is a vote this node
    /// to become the next leader (and create a proposal)
    Accept { accept: ProposalAccept<P> },

    /// Node is missing proposals
    OutOfSync {
        /// Height of the node
        height: u64,
        max_seen_height: u64,
    },

    /// Duplicate proposal received
    DuplicateProposal { proposal_hash: ProposalHash },
}
