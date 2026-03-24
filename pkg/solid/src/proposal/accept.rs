use super::ProposalHeader;
use crate::ProposalAcceptSigData;
use crate::errors::{Error, Result};
use crate::traits::Peer;
use borsh::{BorshDeserialize, BorshSerialize};

/// ProposalAccept is sent by all peers to the next leader to indicate
/// they accept a previous proposal. ProposalAccept is also used in the scenario
/// where a leader is skipped because they did not produce a proposal in time.
#[derive(Debug, Clone, Hash, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ProposalAccept<P: Peer> {
    /// The leader being sent the accept, if the leader collects enough
    /// accepts they can propose
    pub leader_id: P,

    /// Header data for the proposal being accepted, allowing to more easily
    /// ignore out of date accepts
    pub proposal: ProposalHeader,

    /// If skips > 0, we have skipped over a previous leader because they
    /// did not produce a proposal within the allocated period. This is the
    /// number of skips that have occurred since the last confirmed proposal.
    pub skips: u64,

    /// The peer the accept is from
    pub from: P,

    /// Signature of the peer
    pub signature: Vec<u8>,
}

impl<P: Peer> ProposalAccept<P> {
    pub fn verify_signature(&self) -> Result<()> {
        // Check the signature is valid
        // TODO: the signature should verify the entire accept, not just the proposal hash

        if !ProposalAcceptSigData::new(self.proposal.clone(), self.skips)
            .verify(&self.from, &self.signature)
        {
            return Err(Error::InvalidAcceptSignature);
        }

        Ok(())
    }
}
