// lint-long-file-override allow-max-lines=500
pub mod accept;
pub mod cache;
pub mod hash;
pub mod header;
pub mod manifest;
// #[cfg(test)]
// mod tests;

pub use self::accept::*;
pub use self::hash::*;
pub use self::header::*;
pub use self::manifest::*;

use crate::Error;
use crate::errors::Result;
use crate::{config::AcceptThreshold, traits::App};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal<A: App> {
    /// Accepts/votes for this proposal, only received by the leader node
    incoming_accepts: BTreeMap<u64, BTreeMap<A::P, ProposalAccept<A::P>>>,

    /// Hash of the proposal state
    hash: ProposalHash,

    /// State of the proposal that is sent across the network
    pub(crate) manifest: Manifest<A::P, A::State>,

    /// Ordered Validators (in order based on height + skip)
    ordered_validators: Vec<A::P>,

    /// Has an initial accept been sent for this proposal, we keep track of this
    /// to know later if we need to skip.
    pub(crate) initial_accept_sent: bool,

    /// Skips sent for this proposal, skips are sent to the next leader
    /// who should produce a proposal at the next height (this proposals height + 1).
    /// E.g. if this proposal is height 54.0, and skips_sent is 2, then the last accept
    /// sent is to 55.2 (for 54.0)
    pub(crate) skips_sent: u64,

    pub(crate) is_validated: bool,
}

impl<A: App> Proposal<A> {
    pub fn new(manifest: Manifest<A::P, A::State>) -> Self {
        let hash = A::hash(&manifest);
        let ordered_validators = manifest.ordered_validators();

        Self {
            incoming_accepts: BTreeMap::new(),
            hash,
            manifest,
            ordered_validators,
            initial_accept_sent: false,
            skips_sent: 0,
            is_validated: false,
        }
    }

    pub fn manifest(&self) -> &Manifest<A::P, A::State> {
        &self.manifest
    }

    /// Generates a genesis proposal, which uses default values except for peers
    pub fn genesis(existing_peers: Vec<A::P>) -> Self {
        Self::new(Manifest::genesis(existing_peers))
    }

    pub fn hash(&self) -> &ProposalHash {
        &self.hash
    }

    pub fn last_hash(&self) -> &ProposalHash {
        &self.manifest.last_proposal_hash
    }

    /// Height of this proposal
    pub fn height(&self) -> u64 {
        self.manifest.height
    }

    /// Number of skips of leader that have occured since the last leadership order change.
    /// Skips should match the skips sent to this node in the ProposalAccept messages.
    pub fn skips(&self) -> u64 {
        self.manifest.skips
    }

    /// Adds an accept to a proposal, and returns whether a proposal should be generated
    pub fn add_accept(&mut self, accept: ProposalAccept<A::P>, threshold: AcceptThreshold) -> bool {
        // Ignore accepts that are not valid
        match self.validate_accept(&accept) {
            Ok(_) => {}
            Err(_) => {
                return false;
            }
        }

        let skips: u64 = accept.skips;
        let added = self
            .incoming_accepts
            .entry(skips)
            .or_default()
            .insert(accept.from.clone(), accept);
        added.is_none() && self.accept_threshold_breached(skips, threshold)
    }

    pub fn accepts_for_skip(&self, skips: &u64) -> Option<Vec<ProposalAccept<A::P>>> {
        self.incoming_accepts
            .get(skips)
            .map(|p| p.values().cloned().collect::<Vec<_>>())
    }

    /// Checks that we have just enough accepts for meeting the majority
    /// threshold, allowing us to confirm the proposal when majority threshold met,
    /// but only when the threshold is first breached
    pub fn accept_threshold_breached(&self, skips: u64, threshold: AcceptThreshold) -> bool {
        let accepts_len = self
            .incoming_accepts
            .get(&skips)
            .map(|p| p.len())
            .unwrap_or(0);

        threshold.is_exact_breach(accepts_len, self.ordered_validators.len())
    }

    /// Finds the highest skip, where the inverse threshold is met.
    pub fn highest_skip_with_inverse(&self, threshold: AcceptThreshold) -> Option<u64> {
        for (skip, accepts) in self.incoming_accepts.iter().rev() {
            let accepts_len = accepts.len();
            let peers_len = std::cmp::max(self.ordered_validators.len(), 1);

            if threshold.inverse_exceeded(accepts_len, peers_len) {
                return Some(*skip);
            }
        }

        None
    }

    /// Get next accept skip
    pub fn next_accept_skip(&self, threshold: AcceptThreshold, skip: bool) -> u64 {
        if !self.initial_accept_sent && self.skips_sent == 0 {
            return 0;
        }

        let skips_sent_with_skip = if skip {
            self.skips_sent + 1
        } else {
            self.skips_sent
        };

        if let Some(highest_with_inverse) = self.highest_skip_with_inverse(threshold)
            && highest_with_inverse > skips_sent_with_skip
        {
            return highest_with_inverse;
        }

        skips_sent_with_skip
    }

    /// Validate the proposal manifest, these checks are the minimum neccessary
    /// checks and it is expected that additional checks will be performed by
    /// the application impl
    pub fn validate_structure(&self, app: &A, threshold: AcceptThreshold) -> Result<()> {
        // Check we have enough accepts
        if !threshold.is_exceeded(self.manifest.accepts.len(), self.ordered_validators.len()) {
            return Err(Error::InsufficientAcceptsForProposal);
        }

        if !self.manifest.verify::<A>(&self.manifest.leader_id) {
            return Err(Error::InvalidProposalSignature);
        }

        // Check all accepts are valid
        for accept in &self.manifest.accepts {
            // Check the signature is valid
            accept.verify_signature()?;

            // Check the accept leader_id matches the manifest (we cannot check the leader_id is valid
            // until validate_contents)
            if accept.leader_id != self.manifest.leader_id {
                return Err(Error::InvalidAcceptLeader {
                    expected: self.manifest.leader_id.to_string(),
                    got: accept.leader_id.to_string(),
                });
            }

            // Check the accept hash matches the last proposal hash
            if accept.proposal.hash != self.manifest.last_proposal_hash {
                return Err(Error::InvalidAcceptProposalHash);
            }
        }

        // Invalid structure based on app impl
        if !app.validate_structure(self.manifest()) {
            return Err(Error::ProposalInvalidAppStructure);
        }

        Ok(())
    }

    pub fn validate_contents(&self, app: &A, last_confirmed: &Proposal<A>) -> Result<()> {
        // Validate the last proposal hash is last confirmed, in practice this should
        // never be the triggered because next pending fn would never return
        // a current proposal with an invalid decendent, but added as an extra defense
        if *last_confirmed.hash() != self.manifest.last_proposal_hash {
            return Err(Error::ProposalInvalidDecendent);
        }

        // Check the leader is correct, we have to have the last confirmed proposal to check
        // the leader as there may have been a validator set change
        if last_confirmed.manifest.get_leader_for_skip(self.skips()) != self.manifest.leader_id {
            return Err(Error::InvalidProposalLeader);
        }

        // Validate the proposal manifest with app specific logic
        if !app.validate_contents(self.manifest(), &last_confirmed.manifest) {
            return Err(Error::ProposalInvalidAppContent);
        }

        // Validate the accepts
        for accept in &self.manifest.accepts {
            // Validate the accept came from a validator
            if !last_confirmed.manifest().validators.contains(&accept.from) {
                return Err(Error::InvalidAcceptValidator);
            }
        }

        Ok(())
    }

    /// Validate an accept sent for this proposal (NOT accepts attached to the manifest)
    pub fn validate_accept(&self, accept: &ProposalAccept<A::P>) -> Result<()> {
        // Check the signature is valid
        accept.verify_signature()?;

        // Check the accept leader_id is valid
        if self.manifest.get_leader_for_skip(accept.skips) != accept.leader_id {
            return Err(Error::InvalidAcceptLeader {
                expected: self.manifest.get_leader_for_skip(accept.skips).to_string(),
                got: accept.leader_id.to_string(),
            });
        }

        // Check the accept is from a validator
        if !self.manifest().validators.contains(&accept.from) {
            return Err(Error::InvalidAcceptValidator);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{
        app::{TestApp, TestAppTxnState, UncheckedPeerId},
        util::{
            accept, create_manifest, create_manifest_with_accepts, create_proposal,
            genesis_manifest, hash, leader, peer, proposal,
        },
    };

    #[test]
    fn test_get_next_leader() {
        let proposal_0 = create_proposal(0, 0, 1, &genesis_manifest());
        let proposal_1 = create_proposal(1, 0, 1, proposal_0.manifest());
        let proposal_2 = create_proposal(2, 0, 1, proposal_1.manifest());
        let proposal_3 = create_proposal(3, 0, 1, proposal_2.manifest());
        let proposal_4 = create_proposal(4, 0, 1, proposal_3.manifest());
        let proposal_5 = create_proposal(5, 0, 1, proposal_4.manifest());
        let proposal_6 = create_proposal(6, 0, 1, proposal_5.manifest());
        let proposal_7 = create_proposal(7, 0, 1, proposal_6.manifest());
        let proposal_8 = create_proposal(8, 0, 1, proposal_7.manifest());

        assert_eq!(proposal_0.manifest.get_leader_for_skip(0), peer(4));
        assert_eq!(proposal_1.manifest.get_leader_for_skip(0), peer(1));
        assert_eq!(proposal_2.manifest.get_leader_for_skip(0), peer(3));
        assert_eq!(proposal_3.manifest.get_leader_for_skip(0), peer(3));
        assert_eq!(proposal_4.manifest.get_leader_for_skip(0), peer(4));
        assert_eq!(proposal_5.manifest.get_leader_for_skip(0), peer(4));
        assert_eq!(proposal_6.manifest.get_leader_for_skip(0), peer(4));
        assert_eq!(proposal_7.manifest.get_leader_for_skip(0), peer(1));
        assert_eq!(proposal_8.manifest.get_leader_for_skip(0), peer(4));
    }

    #[test]
    fn test_get_next_leader_with_skips() {
        let manifest: Manifest<UncheckedPeerId, TestAppTxnState> = Manifest::new(
            ManifestContent {
                last_proposal_hash: ProposalHash::from_vec_hash(vec![0u8]),
                skips: 0,
                height: 0,
                leader_id: peer(1),
                state: 0.into(),
                validators: vec![peer(1), peer(2), peer(3)],
                accepts: vec![],
            },
            vec![],
        );

        let proposal = Proposal::<TestApp>::new(manifest);

        // Deterministic sort order
        assert_eq!(proposal.ordered_validators, vec![peer(3), peer(2), peer(1)]);

        // Can loop around the peers if needed
        assert_eq!(proposal.manifest.get_leader_for_skip(0), peer(3));
        assert_eq!(proposal.manifest.get_leader_for_skip(1), peer(2));
        assert_eq!(proposal.manifest.get_leader_for_skip(2), peer(1));
        assert_eq!(proposal.manifest.get_leader_for_skip(3), peer(3));
    }

    #[test]
    fn test_two_thirds_accept_breached_4_peers() {
        let manifest = genesis_manifest();
        let mut proposal = proposal(manifest.clone());

        assert!(
            !proposal.accept_threshold_breached(0, AcceptThreshold::MoreThanTwoThirds),
            "Should not be breached"
        );

        proposal.add_accept(
            accept(&manifest, 0, leader(4), peer(1)),
            AcceptThreshold::MoreThanTwoThirds,
        );

        assert!(
            !proposal.accept_threshold_breached(0, AcceptThreshold::MoreThanTwoThirds),
            "Should not be breached"
        );

        proposal.add_accept(
            accept(&manifest, 0, leader(4), peer(2)),
            AcceptThreshold::MoreThanTwoThirds,
        );

        assert!(
            !proposal.accept_threshold_breached(0, AcceptThreshold::MoreThanTwoThirds),
            "Should not be breached"
        );

        proposal.add_accept(
            accept(&manifest, 0, leader(4), peer(3)),
            AcceptThreshold::MoreThanTwoThirds,
        );

        assert!(
            proposal.accept_threshold_breached(0, AcceptThreshold::MoreThanTwoThirds),
            "Should be breached"
        );

        proposal.add_accept(
            accept(&manifest, 0, leader(4), peer(4)),
            AcceptThreshold::MoreThanTwoThirds,
        );

        assert!(
            !proposal.accept_threshold_breached(0, AcceptThreshold::MoreThanTwoThirds),
            "Should not be breached"
        );
    }

    #[test]
    fn valid_proposal() {
        let last_proposal = create_manifest(0, 0, 1, &genesis_manifest());
        let app = TestApp;

        let manifest = create_manifest_with_accepts(
            1,
            0,
            1,
            &hash(&last_proposal),
            vec![
                accept(&last_proposal, 0, peer(1), peer(1)),
                accept(&last_proposal, 0, peer(1), peer(2)),
                accept(&last_proposal, 0, peer(1), peer(3)),
            ],
        );

        assert_eq!(
            Proposal::new(manifest).validate_structure(&app, AcceptThreshold::MoreThanTwoThirds),
            Ok(())
        );
    }

    #[test]
    fn insufficient_accepts_for_proposal() {
        let app = TestApp;
        let manifest = Manifest::new(
            ManifestContent {
                last_proposal_hash: ProposalHash::genesis(),
                skips: 0,
                height: 0,
                leader_id: peer(1),
                state: 0.into(),
                validators: vec![peer(1)],
                accepts: vec![],
            },
            vec![],
        );

        assert_eq!(
            Proposal::new(manifest).validate_structure(&app, AcceptThreshold::MoreThanTwoThirds),
            Err(Error::InsufficientAcceptsForProposal)
        );
    }

    #[test]
    fn invalid_proposal_hash() {
        let app = TestApp;
        let last_proposal = create_manifest(0, 0, 1, &genesis_manifest());
        let alt_proposal = create_manifest(0, 1, 1, &genesis_manifest());

        let manifest = create_manifest_with_accepts(
            1,
            0,
            1,
            &hash(&last_proposal),
            vec![
                accept(&last_proposal, 0, peer(1), peer(1)),
                accept(&last_proposal, 0, peer(1), peer(2)),
                accept(&alt_proposal, 0, peer(1), peer(3)),
            ],
        );

        assert_eq!(
            Proposal::new(manifest).validate_structure(&app, AcceptThreshold::MoreThanTwoThirds),
            Err(Error::InvalidAcceptProposalHash)
        );
    }
}
