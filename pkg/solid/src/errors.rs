use std::fmt::Debug;

use crate::proposal;

pub type Result<T> = std::result::Result<T, Error>;

// TODO: we should pass info with each of these errors

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid signature for accept")]
    InvalidAcceptSignature,

    #[error("invalid accept for proposal")]
    InvalidAcceptProposalHash,

    #[error("invalid validator accept in proposal")]
    InvalidAcceptValidator,

    #[error("invalid proposal leader")]
    InvalidProposalLeader,

    #[error("invalid accept leader, expected: {expected}, got: {got}")]
    InvalidAcceptLeader { expected: String, got: String },

    #[error("insufficient accepts for proposal")]
    InsufficientAcceptsForProposal,

    #[error("invalid signature for proposal")]
    InvalidProposalSignature,

    #[error("proposal already exists")]
    ProposalAlreadyExists(proposal::ProposalHash),

    #[error("proposal height too low")]
    ProposalHeightTooLow,

    #[error("proposal peer threshold not met")]
    ProposalPeerThresholdNotMet,

    #[error("confirmed proposal is not a decendent")]
    ProposalInvalidDecendent,

    #[error("proposal invalid structure")]
    ProposalInvalidAppStructure,

    #[error("proposal invalid content")]
    ProposalInvalidAppContent,
}
