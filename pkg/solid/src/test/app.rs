use crate::{
    AppState,
    proposal::{ManifestContent, ProposalHash},
    traits::{App, Peer, PeerSigner},
    util::u256::U256,
};
use borsh::{BorshDeserialize, BorshSerialize};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::borrow::Borrow;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestApp;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Default)]
pub struct TestAppTxnState(u64);

impl From<u64> for TestAppTxnState {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

impl AppState for TestAppTxnState {}

impl App for TestApp {
    type State = TestAppTxnState;
    type P = UncheckedPeerId;
    type PS = InsecurePeerSigner;

    fn hash(manifest: &ManifestContent<Self::P, Self::State>) -> ProposalHash {
        #[allow(clippy::unwrap_used)]
        #[allow(clippy::disallowed_methods)]
        // we are only serializing, so no format requirements
        let bytes = Sha256::digest(borsh::to_vec(&manifest).unwrap());
        ProposalHash::new(bytes.into())
    }
}

#[derive(
    Default, Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash, BorshSerialize, BorshDeserialize,
)]
pub struct UncheckedPeerId(pub Vec<u8>);

impl Peer for UncheckedPeerId {
    fn verify(&self, _signature: &[u8], _msg: [u8; 32]) -> bool {
        true
    }

    fn prefix(&self) -> String {
        let string = self.to_string();
        if string.len() > 4 {
            string[string.len() - 4..].to_string()
        } else {
            string
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    fn genesis() -> Self {
        Self(vec![0u8])
    }

    fn to_u256(&self) -> U256 {
        U256::from_little_endian(&self.0)
    }
}

impl UncheckedPeerId {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn random() -> UncheckedPeerId {
        let peer_id = rand::thread_rng().r#gen::<[u8; 32]>();
        UncheckedPeerId(peer_id.to_vec())
    }
}

impl Display for UncheckedPeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(&self.0).into_string())
    }
}

impl Borrow<[u8]> for UncheckedPeerId {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

impl From<&[u8]> for UncheckedPeerId {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

#[derive(Debug, Clone)]
pub struct InsecurePeerSigner {
    peer: UncheckedPeerId,
}

impl InsecurePeerSigner {
    pub fn new(peer: UncheckedPeerId) -> Self {
        Self { peer }
    }
}

impl PeerSigner<UncheckedPeerId> for InsecurePeerSigner {
    fn sign(&self, _proposal: [u8; 32]) -> Vec<u8> {
        vec![]
    }

    fn peer(&self) -> UncheckedPeerId {
        self.peer.clone()
    }
}
