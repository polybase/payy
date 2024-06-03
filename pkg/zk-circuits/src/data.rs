use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use smirk::Element;

use crate::{aggregate_utxo::AggregateUtxo, Snark, UTXO_INPUTS, UTXO_OUTPUTS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterSet {
    Six,
    Eight,
    Nine,
    Fourteen,
    TwentyOne,
}

#[derive(Clone, Debug)]
pub struct Burn<const L: usize> {
    pub secret_key: Element,
    pub notes: [Note; L],
    pub to_address: Element,
}

// https://github.com/rust-lang/rust/issues/61415
impl<const L: usize> Default for Burn<L> {
    fn default() -> Self {
        Self {
            secret_key: Element::default(),
            notes: core::array::from_fn(|_| Note::default()),
            to_address: Element::default(),
        }
    }
}

// TODO: change Fr to Element
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Note {
    /// Address of owner of the note (AKA nullifer key or nk, a commitment to the secret key)
    pub address: Element,
    /// Blake2 hash with salts for increased entropy
    pub psi: Element,
    /// Value of the note
    pub value: Element,
    /// Kind of note
    pub token: String,
    /// Source of note (should be ethereum address)
    pub source: Element,
}

#[derive(Clone, Debug)]
pub struct Mint<const L: usize> {
    pub notes: [Note; L],
}

// https://github.com/rust-lang/rust/issues/61415
impl<const L: usize> Default for Mint<L> {
    fn default() -> Self {
        Self {
            notes: [(); L].map(|_| Note::default()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Insert<const MERKLE_D: usize> {
    /// Leaf node
    pub leaf: Element,
    /// Sibling path (does not include leaf or root)
    pub path: MerklePath<MERKLE_D>,
}

/// The siblings of a merkle path, for a [`smirk::Tree`] of depth `DEPTH`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerklePath<const DEPTH: usize> {
    /// The siblings that form the merkle path
    pub siblings: Vec<Element>,
}

impl<const DEPTH: usize> Default for MerklePath<DEPTH> {
    fn default() -> Self {
        let siblings = (1..DEPTH).map(smirk::empty_tree_hash).collect::<Vec<_>>();

        assert_eq!(siblings.len(), DEPTH - 1);

        Self { siblings }
    }
}

#[derive(Clone, Debug)]
pub struct Batch<const INSERTS: usize, const MERKLE_D: usize> {
    /// Inserts must link to each other, in other words the new root of the first element must match
    /// the old root of the second element, and so on.
    pub inserts: [Insert<MERKLE_D>; INSERTS],
}

impl<const INSERTS: usize, const MERKLE_D: usize> Default for Batch<INSERTS, MERKLE_D> {
    fn default() -> Self {
        Self {
            inserts: core::array::from_fn(|_| Insert::default()),
        }
    }
}

/// InputNote is a Note that belongs to the current user, i.e. they have the
/// spending sercret key and can therefore use it as an input, "spending" the note. Extra
/// constraints need to be applied to input notes to ensure they are valid.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InputNote<const MERKLE_D: usize> {
    pub note: Note,
    /// Secret key for the address, required to spend a note
    pub secret_key: Element,
    /// Input notes merkle tree path, so we can verify that the note exists
    /// in the tree, without revealing which hash it is
    /// Path for tree that matches recent root
    pub merkle_path: MerklePath<MERKLE_D>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Utxo<const MERKLE_D: usize> {
    pub inputs: [InputNote<MERKLE_D>; UTXO_INPUTS],
    pub outputs: [Note; UTXO_OUTPUTS],

    /// Merkle root of the input notes (required to prove that input notes already
    /// exist in the tree and can therefore be spent)
    pub root: Element,

    // Kind of transaction
    pub kind: UtxoKind,
}

impl<const MERKLE_D: usize> Default for Utxo<MERKLE_D> {
    fn default() -> Self {
        Self {
            inputs: core::array::from_fn(|_| InputNote::default()),
            outputs: core::array::from_fn(|_| Note::default()),
            root: Element::ZERO,
            kind: UtxoKind::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UtxoKind {
    Null,
    #[default]
    Transfer,
    Mint,
    Burn,
}

#[derive(
    Debug,
    Default,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct UTXOProof<const MERKLE_D: usize> {
    /// Root hash
    pub recent_root: Element,
    /// Mint/Burn hash (null for transfer)
    pub mb_hash: Element,
    /// Mint/Burn value (null for transfer)
    pub mb_value: Element,
    /// Leaves
    pub input_leaves: [Element; UTXO_INPUTS],
    pub output_leaves: [Element; UTXO_OUTPUTS],
    /// Proof
    pub proof: Vec<u8>,
}

/// The serialized form of a proof
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wire_message::wire_message]
pub enum SnarkWitness {
    V1(SnarkWitnessV1),
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct SnarkWitnessV1 {
    pub instances: Vec<Vec<Element>>,
    #[serde(
        serialize_with = "crate::util::serialize_base64",
        deserialize_with = "crate::util::deserialize_base64"
    )]
    pub proof: Vec<u8>,
}

impl wire_message::WireMessage for SnarkWitness {
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        match self {
            Self::V1(_) => 1,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        Err(Self::max_version_error())
    }
}

#[derive(Clone, Default, Debug)]
pub struct Signature {
    /// Secret key for the address, required to spend a note
    pub secret_key: Element,
    /// Message to be signed
    pub message: Element,
}

#[derive(Clone, Debug)]
pub struct Points {
    /// Secret key
    pub secret_key: Element,
    /// Message to be signed
    pub notes: Vec<Note>,
}

#[derive(Clone, Debug)]
pub struct AggregateAgg<const AGG_N: usize> {
    /// UTXO to aggregate
    pub aggregates: [Snark; AGG_N],

    /// Instances used to verify the proof
    pub agg_instances: Vec<Element>,

    /// Private witness to proof
    pub proof: Vec<u8>,
}

impl<const AGG_N: usize> Default for AggregateAgg<AGG_N> {
    fn default() -> Self {
        let aggregate_utxo = AggregateUtxo::<3, 161, 12>::default()
            .snark(ParameterSet::TwentyOne)
            .unwrap();

        Self::new(core::array::from_fn(|_| aggregate_utxo.clone()))
    }
}
