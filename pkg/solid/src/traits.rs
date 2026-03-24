use crate::proposal::{ManifestContent, ProposalHash, ProposalHeader};
use crate::util::u256::U256;
use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Keccak256};
use std::fmt::{Debug, Display};
use std::hash::Hash;

pub trait AppState:
    Debug + Clone + Default + Send + PartialEq + Eq + BorshSerialize + BorshDeserialize + 'static
{
    fn genesis() -> Self {
        Self::default()
    }
}

pub trait App: Clone + Debug + Send + Sync + 'static {
    /// The state for a txn
    type State: AppState;

    /// An external peer address, some other peer on the network
    type P: Peer;

    /// PeerSigner, used by the running node to sign proposals
    type PS: PeerSigner<Self::P>;

    /// Validate the proposal structure, this validates that the basic structure of the proposal
    /// is valid. It validates whether a proposal COULD be valid.
    fn validate_structure(&self, _: &ManifestContent<Self::P, Self::State>) -> bool {
        true
    }

    /// Validate the proposal contents, this called when the proposal is next inline to be
    /// confirmed. It validates whether a proposal IS valid based on previous state of the app.
    fn validate_contents(
        &self,
        _manifest: &ManifestContent<Self::P, Self::State>,
        _last_confirmed: &ManifestContent<Self::P, Self::State>,
    ) -> bool {
        true
    }

    /// Hash of the proposal/block
    fn hash(manifest: &ManifestContent<Self::P, Self::State>) -> ProposalHash;
}

#[derive(Debug, Clone)]
pub struct ProposalAcceptSigData {
    proposal: ProposalHeader,
    skips: u64,
}

impl ProposalAcceptSigData {
    pub fn new(proposal: ProposalHeader, skips: u64) -> Self {
        Self { proposal, skips }
    }

    fn hash(&self) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        hasher.update(&self.proposal.hash.inner()[..]);
        hasher.update(self.skips.to_be_bytes());

        hasher.finalize().into()
    }

    pub fn sign<P: Peer>(&self, signer: &impl PeerSigner<P>) -> Vec<u8> {
        signer.sign(self.hash())
    }

    pub fn verify<P: Peer>(&self, peer: &P, signature: &[u8]) -> bool {
        peer.verify(signature, self.hash())
    }
}

pub trait Peer:
    Default
    + Debug
    + Display
    + Clone
    + PartialEq
    + Ord
    + PartialOrd
    + Eq
    + Hash
    + BorshSerialize
    + BorshDeserialize
    + Send
    + Sync
    + 'static
{
    fn verify(&self, signature: &[u8], msg: [u8; 32]) -> bool;

    fn prefix(&self) -> String;

    fn to_bytes(&self) -> Vec<u8>;

    fn genesis() -> Self {
        Self::default()
    }

    fn to_u256(&self) -> U256;

    fn distance(&self, other: &U256) -> U256 {
        *other ^ self.to_u256()
    }
}

pub trait PeerSigner<P: Peer>: Clone + Debug + Send + Sync + 'static {
    fn sign(&self, msg: [u8; 32]) -> Vec<u8>;

    fn peer(&self) -> P;
}
