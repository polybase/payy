use std::ops::{Deref, DerefMut};

use super::{ProposalAccept, ProposalHash};
use crate::traits::{AppState, Peer};
use crate::util::u256;
use crate::{App, PeerSigner};
use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};

#[derive(Default, Debug, Clone, Hash, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Manifest<P: Peer, S: AppState> {
    pub content: ManifestContent<P, S>,
    pub signature: Vec<u8>,
}

impl<P: Peer, S: AppState> Manifest<P, S> {
    pub fn new(content: ManifestContent<P, S>, signature: Vec<u8>) -> Self {
        Manifest { content, signature }
    }

    pub fn genesis(validators: Vec<P>) -> Self {
        Manifest {
            content: ManifestContent::genesis(validators),
            signature: vec![],
        }
    }

    pub fn verify<A: App<P = P, State = S>>(&self, signer: &P) -> bool {
        signer.verify(&self.signature, *A::hash(&self.content).inner())
    }
}

impl<P: Peer, S: AppState> Deref for Manifest<P, S> {
    type Target = ManifestContent<P, S>;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<P: Peer, S: AppState> DerefMut for Manifest<P, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

#[derive(Default, Debug, Clone, Hash, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ManifestContent<P: Peer, S: AppState> {
    /// Hash of the last proposal, so we can confirm the last
    /// proposal when we receive this message
    pub last_proposal_hash: ProposalHash,

    /// Number of skips of leader that have occured since the last
    /// leadership order change. This skips should match the skips
    /// sent to this node in the ProposalAccept messages.
    pub skips: u64,

    /// Height of the proposal, for easy checking whether we
    /// are up to date with the network
    pub height: u64,

    /// PeerId of the proposer/leader
    pub leader_id: P,

    /// Changes included in the proposal
    pub state: S,

    /// List of validators on the network
    pub validators: Vec<P>,

    /// List of collected accepts for last proposal hash
    pub accepts: Vec<ProposalAccept<P>>,
}

impl<P: Peer, S: AppState> ManifestContent<P, S> {
    fn genesis(validators: Vec<P>) -> Self {
        let leader_id = get_leader_for_skip(0, &validators);
        ManifestContent {
            last_proposal_hash: ProposalHash::genesis(),
            skips: 0,
            height: 0,
            leader_id,
            state: S::genesis(),
            validators,
            accepts: vec![],
        }
    }

    pub fn get_leader_for_skip(&self, skip: u64) -> P {
        get_leader_for_skip(skip, &self.ordered_validators())
    }

    /// Get a list of ordered validators, this is a little expensive so it
    /// can be useful to cache it
    pub fn ordered_validators(&self) -> Vec<P> {
        let mut validators = self.validators.to_vec();
        validators.sort_by_key(|a| a.distance(&ordering_hash(self.height, self.skips)));
        validators
    }

    pub fn sign<A: App<P = P, State = S>>(&self, signer: &A::PS) -> Vec<u8> {
        signer.sign(*A::hash(self).inner())
    }
}

fn get_leader_for_skip<P: Peer>(skip: u64, ordered_validators: &[P]) -> P {
    // We can use usize cast here, because skips reset for each proposal. Given min 1s per proposal skip,
    // even with 2^32 arch it would take 136 years to overflow
    let pos = (skip as usize) % ordered_validators.len();
    let peer = &ordered_validators[pos];
    peer.clone()
}

/// Used to order peers in a safe way, its not possible for nodes to manipulate
/// the height/skips as these are what determine the previous leader
fn ordering_hash(height: u64, skips: u64) -> u256::U256 {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&skips.to_be_bytes());
    let hash = Sha256::digest(bytes);
    u256::U256::from_little_endian(hash.as_slice())
}
