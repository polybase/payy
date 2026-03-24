// lint-long-file-override allow-max-lines=300
use block_store::BlockStore;
use borsh::{BorshDeserialize, BorshSerialize};
use contextful::ErrorContextExt as _;
use element::Element;
use ethereum_types::U256;
use node_interface::{ElementsVecData, RpcError};
use parking_lot::RwLock;
use primitives::{hash::CryptoHash, peer::PeerIdSigner};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::{collections::HashMap, fmt::Debug};
use tracing::error;
use zk_circuits::BbBackend;

use crate::types::BlockHeight;
use crate::utxo::{validate_txn_state, verify_txn_proof};
use crate::{BlockFormat, PersistentMerkleTree};
use crate::{Error, Mode};
use primitives::sig::Signature;
use zk_primitives::UtxoProof;

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct Block {
    pub content: BlockContent,
    pub signature: Signature,
}

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct BlockContent {
    pub header: BlockHeader,
    pub state: BlockState,
}

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct BlockHeader {
    pub height: BlockHeight,
    pub last_block_hash: CryptoHash,
    pub epoch_id: u64,
    pub last_final_block_hash: CryptoHash,
    pub approvals: Vec<Signature>,
}

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct BlockState {
    pub root_hash: Element,
    pub txns: Vec<UtxoProof>,
}

impl BlockState {
    pub fn new(root_hash: Element, txns: Vec<UtxoProof>) -> Self {
        Self { root_hash, txns }
    }

    // /// Get an iterator over all leaves in this block
    // pub fn leaves(&self) -> impl Iterator<Item = Element> + '_ {
    //     self.txns.iter().flat_map(|proof| proof.leaves())
    // }

    // /// Get an iterator over all the non-null leaves in this block
    // pub fn leaves_non_null(&self) -> impl Iterator<Item = Element> + '_ {
    //     self.leaves().filter(|&e| e != Element::NULL_HASH)
    // }
}

impl Block {
    pub fn genesis() -> Block {
        Block {
            content: BlockContent::genesis(),
            signature: Signature::default(),
        }
    }

    pub fn hash(&self) -> CryptoHash {
        self.content.hash()
    }
}

impl BlockContent {
    pub fn genesis() -> BlockContent {
        BlockContent {
            header: BlockHeader {
                height: BlockHeight(0),
                last_block_hash: CryptoHash::default(),
                epoch_id: 0,
                last_final_block_hash: CryptoHash::default(),
                approvals: vec![],
            },
            state: BlockState::default(),
        }
    }

    pub fn to_block(&self, peer: &PeerIdSigner) -> Block {
        Block {
            content: self.clone(),
            signature: self.sign(peer),
        }
    }

    #[allow(clippy::disallowed_methods)]
    pub fn header_hash(&self) -> CryptoHash {
        let mut hasher = Keccak256::new();
        hasher.update(borsh::to_vec(&self.header).unwrap());
        CryptoHash::new(hasher.finalize().into())
    }

    #[allow(clippy::disallowed_methods)] // not deserializing, so upgradable format isn't needed
    pub fn hash(&self) -> CryptoHash {
        // To be verified by Ethereum smart contract
        let root_hash = self.state.root_hash.to_be_bytes();

        // TODO: use merkle_patricia_tree to hash all manifest items properly

        let header_hash: [u8; 32] = *self.header_hash().inner();

        let mut hasher = Keccak256::new();
        hasher.update(root_hash);

        let mut height_bytes = [0u8; 32];
        U256::from(self.header.height.0).to_big_endian(&mut height_bytes);
        hasher.update(height_bytes);

        hasher.update(header_hash);
        let result = hasher.finalize();

        CryptoHash::new(result.into())
    }

    pub fn sign(&self, signer: &PeerIdSigner) -> Signature {
        signer.sign(&self.hash())
    }

    pub(crate) async fn validate(
        &self,
        bb_backend: &dyn BbBackend,
        mode: Mode,
        block_store: &BlockStore<BlockFormat>,
        notes_tree: &RwLock<PersistentMerkleTree>,
    ) -> Result<(), Error> {
        let mut insert_txn_leaves = HashMap::new();
        let mut remove_txn_leaves = HashMap::new();

        // 1. Verify all proofs first (async, without lock)
        for utxo_proof in self.state.txns.iter() {
            verify_txn_proof(bb_backend, utxo_proof).await?;
        }

        // 2. Take lock and validate all states (sync)
        let notes_tree_guard = notes_tree.read();

        for utxo_proof in self.state.txns.iter() {
            // Between transactions in the same block,
            // we check that the leaves are unique,
            // otherwise there could be a double spend.
            for leaf in utxo_proof.public_inputs.output_commitments {
                if leaf == Element::ZERO {
                    continue;
                }

                let existing_leaf_txn_hash = insert_txn_leaves
                    .get(&leaf)
                    .or(remove_txn_leaves.get(&leaf));
                if existing_leaf_txn_hash.is_some() {
                    return Err(RpcError::ConflictingElementsInBlock(ElementsVecData {
                        elements: vec![leaf],
                    })
                    .wrap_err("duplicate output commitment detected within block")
                    .into());
                } else {
                    insert_txn_leaves.insert(leaf, utxo_proof.hash());
                }
            }

            for leaf in utxo_proof.public_inputs.input_commitments {
                if leaf == Element::ZERO {
                    continue;
                }

                let existing_leaf_txn_hash = insert_txn_leaves
                    .get(&leaf)
                    .or(remove_txn_leaves.get(&leaf));
                if existing_leaf_txn_hash.is_some() {
                    return Err(RpcError::ConflictingElementsInBlock(ElementsVecData {
                        elements: vec![leaf],
                    })
                    .wrap_err("duplicate input commitment detected within block")
                    .into());
                } else {
                    remove_txn_leaves.insert(leaf, utxo_proof.hash());
                }
            }

            let result = validate_txn_state(
                mode,
                utxo_proof,
                // TODO: is this valid?
                self.header.height,
                block_store,
                &notes_tree_guard,
            );

            if let Err(err) = result {
                error!(
                    ?err,
                    height = ?self.header.height,
                    hash = ?self.hash(),
                    "Failed to validate transaction in received proposal"
                );
                return Err(err);
            }
        }

        let new_root_hash = if insert_txn_leaves.is_empty() && remove_txn_leaves.is_empty() {
            // If there is no leaves to insert, the root hash wouldn't change
            notes_tree_guard.tree().root_hash()
        } else {
            notes_tree_guard.tree().root_hash_with(
                &insert_txn_leaves.into_keys().collect::<Vec<_>>(),
                &remove_txn_leaves.into_keys().collect::<Vec<_>>(),
            )
        };
        if new_root_hash != self.state.root_hash {
            return Err(Error::InvalidBlockRoot {
                got: new_root_hash,
                expected: self.state.root_hash,
            });
        }

        Ok(())
    }
}
