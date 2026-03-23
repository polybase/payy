#![warn(clippy::unwrap_used, clippy::expect_used)]
#![deny(clippy::disallowed_methods)]

// lint-long-file-override allow-max-lines=600
mod constants;
pub mod smirk_metadata;
use crate::constants::{
    MERKLE_TREE_DEPTH, MERKLE_TREE_PATH_DEPTH, UTXO_AGG_LEAVES, UTXO_AGG_NUMBER, UTXO_AGGREGATIONS,
};
use borsh::{BorshDeserialize, BorshSerialize};
pub use constants::MAXIMUM_TXNS;
use contracts::RollupContract;
use element::Element;
use ethereum_types::H256;
use primitives::sig::Signature;
use smirk::{
    Path, Tree,
    hash_cache::{NoopHashCache, SimpleHashCache},
};
use smirk_metadata::SmirkMetadata;
use std::sync::Arc;
use tracing::info;
use web3::{ethabi, types::TransactionId};
use zk_circuits::{BbBackend, Prove, Verify};
use zk_primitives::{
    AggAgg, AggFinal, AggFinalProof, AggProof, AggUtxo, AggUtxoProof, MerklePath, UtxoProof,
    UtxoProofBundleWithMerkleProofs,
};

type Result<T, E = Error> = std::result::Result<T, E>;

type MerkleTree<C = NoopHashCache> = Tree<MERKLE_TREE_DEPTH, SmirkMetadata, C>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to convert H256 to bn256::Fr")]
    ConvertH256ToBn256Fr(H256),

    // Temporary workaround, sometimes rollup transactions get stuck. Restarting the prover fixes it
    #[error("rollup transaction timed out")]
    RollupTransactionTimeout,

    #[error("from hex error")]
    FromHex(#[from] rustc_hex::FromHexError),

    #[error("web3 error")]
    Web3(#[from] web3::Error),

    #[error("ethabi error")]
    EthAbi(#[from] ethabi::Error),

    #[error("TryFromSlice error")]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    #[error("web3 contract error")]
    Web3Contract(#[from] web3::contract::Error),

    #[error("serde_json error")]
    SerdeJson(#[from] serde_json::Error),

    #[error("secp256k1 error")]
    Secp256k1(#[from] secp256k1::Error),

    #[error("smirk storage error")]
    Smirk(#[from] smirk::storage::Error),

    #[error("smirk collision error")]
    SmirkCollision(#[from] smirk::CollisionError),

    #[error("contract error")]
    Contract(#[from] contracts::Error),

    #[error("tokio task join error")]
    TokioTaskJoin(#[from] tokio::task::JoinError),

    #[error("barretenberg prove error: {0}")]
    BarretenbergProve(String),

    #[error("vec to array conversion failed, expected {expected}, got {actual}")]
    VecToArrayConversion { expected: usize, actual: usize },
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub proof: UtxoProof,
}

impl Transaction {
    pub fn new(proof: UtxoProof) -> Self {
        Self { proof }
    }
}

#[derive(Debug, Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct RollupInput {
    proof: AggFinalProof,
    height: u64,
    other_hash: [u8; 32],
    signatures: Vec<Signature>,
}

impl RollupInput {
    pub fn new(
        proof: AggFinalProof,
        height: u64,
        other_hash: [u8; 32],
        signatures: Vec<Signature>,
    ) -> Self {
        Self {
            proof,
            height,
            other_hash,
            signatures,
        }
    }

    pub fn old_root(&self) -> Element {
        self.proof.public_inputs.old_root
    }

    pub fn new_root(&self) -> Element {
        self.proof.public_inputs.new_root
    }

    pub fn height(&self) -> u64 {
        self.height
    }
}

pub struct Prover {
    contract: RollupContract,
    bb_backend: Arc<dyn BbBackend>,
}

impl Prover {
    pub fn new(contract: RollupContract, bb_backend: Arc<dyn BbBackend>) -> Self {
        Self {
            contract,
            bb_backend,
        }
    }

    #[tracing::instrument(err, skip_all, fields(height, txns_len = txns.len()))]
    pub async fn prove(
        self: &Arc<Self>,
        notes_tree: &MerkleTree<SimpleHashCache>,
        height: u64,
        txns: [Option<Transaction>; MAXIMUM_TXNS],
    ) -> Result<AggFinalProof> {
        info!(
            "Bundling {} UTXO proof(s) and proving new root hash",
            txns.len()
        );

        let mut tree = notes_tree.clone();
        let proof = self
            .generate_aggregate_proof(&mut tree, txns, height)
            .await?;

        Ok(proof)
    }

    #[tracing::instrument(err, skip(self), fields(height = input.height))]
    pub async fn rollup(&self, input: &RollupInput) -> Result<H256> {
        info!("Sending proof and new root to Ethereum");

        let tx = self
            .contract
            .verify_block(
                &input.proof.proof.0,
                &input.proof.public_inputs.old_root,
                &input.proof.public_inputs.new_root,
                &input.proof.public_inputs.commit_hash,
                &input.proof.public_inputs.messages.to_vec(),
                &input.proof.kzg,
                input.other_hash,
                input.height,
                &input
                    .signatures
                    .iter()
                    .map(|s| &s.0[..])
                    .collect::<Vec<_>>(),
                1_000_000,
            )
            .await?;

        info!(
            ?tx,
            "Ethereum root rollup update sent. Waiting for receipt...",
        );

        let wait_start = std::time::Instant::now();
        while self
            .contract
            .client
            .client()
            .eth()
            .transaction_receipt(tx)
            .await?
            .is_none()
        {
            if self
                .contract
                .client
                .client()
                .eth()
                .transaction(TransactionId::Hash(tx))
                .await?
                .is_none()
            {
                // The transaction was dropped
                break;
            }

            // Wait for a maximum of 5 minutes
            if wait_start.elapsed() > std::time::Duration::from_secs(5 * 60) {
                return Err(Error::RollupTransactionTimeout);
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        info!("Ethereum root rollup update confirmed");

        Ok(tx)
    }

    #[tracing::instrument(err, skip_all)]
    async fn generate_aggregate_proof(
        &self,
        tree: &mut MerkleTree<SimpleHashCache>,
        txns: [Option<Transaction>; 6],
        current_block: u64,
    ) -> Result<AggFinalProof, Error> {
        let txns = txns
            .into_iter()
            .map(|t| match t {
                Some(t) => Ok(t),
                None => Ok(Transaction {
                    proof: UtxoProof::default(),
                }),
            })
            .collect::<Result<Vec<_>>>()?;
        let txns = &mut txns.iter();

        let mut utxo_aggregations = Vec::new();
        for _i in 0..UTXO_AGGREGATIONS {
            // Take the first 3 txns (removing from the vec)
            // Unwrap is safe because we know we have enough txns
            #[allow(clippy::unwrap_used)]
            let txns: [Transaction; UTXO_AGG_NUMBER] = txns
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            let utxo_aggregate = self
                .aggregate_utxo(tree, txns.clone(), current_block)
                .await?;
            utxo_aggregations.push(utxo_aggregate);
        }

        let utxo_aggregations: [AggUtxoProof; UTXO_AGGREGATIONS] = utxo_aggregations
            .try_into()
            .map_err(|v: Vec<_>| Error::VecToArrayConversion {
                expected: UTXO_AGGREGATIONS,
                actual: v.len(),
            })?;

        let agg_proofs: [AggProof; UTXO_AGGREGATIONS] =
            utxo_aggregations.map(|proof| AggProof::AggUtxo(Box::new(proof)));

        let agg_agg = AggAgg::new(agg_proofs);
        let agg_agg_proof = agg_agg
            .prove(&*self.bb_backend)
            .await
            .map_err(|e| Error::BarretenbergProve(e.to_string()))?;

        let agg_final = AggFinal::new(agg_agg_proof);
        let proof = agg_final
            .prove(&*self.bb_backend)
            .await
            .map_err(|e| Error::BarretenbergProve(e.to_string()))?;

        // Verify the newly generated proof
        proof
            .verify(&*self.bb_backend)
            .await
            .map_err(|e| Error::BarretenbergProve(e.to_string()))?;

        Ok(proof)
    }

    #[tracing::instrument(err, skip_all)]
    async fn aggregate_utxo(
        &self,
        tree: &mut MerkleTree<SimpleHashCache>,
        utxos: [Transaction; UTXO_AGG_NUMBER],
        current_block: u64,
    ) -> Result<AggUtxoProof, Error> {
        if utxos.iter().all(|utxo| utxo.proof.is_padding()) {
            return Ok(AggUtxoProof::default());
        }

        let (_, old_tree, new_tree, merkle_paths) =
            self.gen_merkle_paths(tree, &utxos, current_block)?;

        // Chunk the merkle paths into 3 x 4
        let merkle_paths = merkle_paths.chunks(4).collect::<Vec<_>>();

        let mut utxo_proof_bundles = Vec::new();

        for (i, chunk) in merkle_paths.into_iter().enumerate() {
            let utxo = &utxos[i];

            let utxo_proof_bundle = match utxo.proof.is_padding() {
                false => UtxoProofBundleWithMerkleProofs::new(
                    utxo.proof.clone(),
                    &[
                        chunk[0].clone(),
                        chunk[1].clone(),
                        chunk[2].clone(),
                        chunk[3].clone(),
                    ],
                ),
                true => UtxoProofBundleWithMerkleProofs::default(),
            };

            utxo_proof_bundles.push(utxo_proof_bundle);
        }

        let utxo_proof_bundles: [UtxoProofBundleWithMerkleProofs; UTXO_AGG_NUMBER] =
            utxo_proof_bundles
                .try_into()
                .map_err(|v: Vec<_>| Error::VecToArrayConversion {
                    expected: UTXO_AGG_NUMBER,
                    actual: v.len(),
                })?;

        let agg_utxo = AggUtxo::new(utxo_proof_bundles, old_tree, new_tree);

        let agg_utxo_proof = agg_utxo
            .prove(&*self.bb_backend)
            .await
            .map_err(|e| Error::BarretenbergProve(e.to_string()))?;

        Ok(agg_utxo_proof)
    }

    #[tracing::instrument(err, skip_all)]
    fn gen_merkle_paths(
        &self,
        tree: &mut MerkleTree<SimpleHashCache>,
        txns: &[Transaction; UTXO_AGG_NUMBER],
        current_block: u64,
    ) -> Result<(
        usize,
        Element,
        Element,
        [MerklePath<MERKLE_TREE_DEPTH>; UTXO_AGG_LEAVES],
    )> {
        let padding_path = MerklePath::default();

        let (merkle_paths, old_tree, new_tree) = {
            let old_tree = tree.root_hash();

            let mut merkle_paths = vec![];

            // Extract leaves to be inserted from proof
            for Transaction { proof } in txns {
                for leaf in proof.public_inputs.input_commitments {
                    if leaf.is_zero() {
                        merkle_paths.push(padding_path.clone());
                        continue;
                    }

                    merkle_paths.push(path_to_merkle_path(tree.path_for(leaf)));
                    tree.remove(leaf)?;
                }

                for leaf in proof.public_inputs.output_commitments {
                    if leaf.is_zero() {
                        merkle_paths.push(padding_path.clone());
                        continue;
                    }

                    // Insert the leaf into the tree
                    tree.insert(
                        leaf,
                        SmirkMetadata {
                            inserted_in: current_block,
                        },
                    )?;
                    // Then add the path to the merkle paths
                    merkle_paths.push(path_to_merkle_path(tree.path_for(leaf)));
                }
            }

            let new_tree = tree.root_hash();

            (merkle_paths, old_tree, new_tree)
        };
        let inserts_len: usize = merkle_paths.len();
        let merkle_paths: [MerklePath<MERKLE_TREE_DEPTH>; UTXO_AGG_LEAVES] = merkle_paths
            .try_into()
            .map_err(|v: Vec<_>| Error::VecToArrayConversion {
                expected: UTXO_AGG_LEAVES,
                actual: v.len(),
            })?;
        Ok((inserts_len, old_tree, new_tree, merkle_paths))
    }

    // pub fn get_merkle_path_for_commitment(
    //     &self,
    //     notes_tree: &MerkleTree,
    //     commitment: Base,
    // ) -> Result<Vec<Base>, Error> {
    //     let el = Element::from_base(commitment);

    //     // Sibling path
    //     let path = notes_tree.path_for(el);

    //     let path = path
    //         .siblings_deepest_first()
    //         .iter()
    //         .copied()
    //         .map(Element::to_base)
    //         .collect();

    //     Ok(path)
    // }
}

fn path_to_merkle_path(path: Path) -> MerklePath<MERKLE_TREE_DEPTH> {
    let elements = path
        .siblings_deepest_first()
        .iter()
        .cloned()
        .take(MERKLE_TREE_PATH_DEPTH)
        .collect::<Vec<Element>>();

    MerklePath::new(elements)
}

// #[cfg(test)]
// mod tests {
//     use std::str::FromStr;

//     use web3::types::Address;
//     use zk_circuits::utxo::{InputNote, Note, Utxo, UtxoKind};

//     use super::*;

//     struct Env {
//         rollup_contract_addr: Address,
//         evm_secret_key: SecretKey,
//     }

//     fn get_env() -> Env {
//         Env {
//             rollup_contract_addr: Address::from_str(
//                 &std::env::var("ROLLUP_CONTRACT_ADDR")
//                     .expect("env var ROLLUP_CONTRACT_ADDR is not set"),
//             )
//             .unwrap(),
//             evm_secret_key: SecretKey::from_str(&std::env::var("PROVER_SECRET_KEY").unwrap_or(
//                 // Seems to be the default when deploying with hardhat to a local node
//                 "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_owned(),
//             ))
//             .unwrap(),
//         }
//     }

//     fn contract(addr: Address) -> RollupContract {
//         let rpc = std::env::var("ETHEREUM_RPC").unwrap_or("http://localhost:8545".to_owned());
//         let web3 = web3::Web3::new(web3::transports::Http::new(&rpc).unwrap());
//         let contract = include_bytes!("../../../eth/artifacts/contracts/Rollup.sol/Rollup.json");
//         let contract = serde_json::from_slice::<serde_json::Value>(contract).unwrap();
//         let contract =
//             serde_json::from_value::<ethabi::Contract>(contract.get("abi").unwrap().clone())
//                 .unwrap();
//         let contract = web3::contract::Contract::new(web3.eth(), addr, contract);

//         RollupContract::new(web3, contract)
//     }

//     #[tokio::test]
//     async fn test_prover() {
//         let env = get_env();
//         let contract = contract(env.rollup_contract_addr);

//         let notes_tree: Arc<Mutex<Tree<33>>> =
//             Arc::new(Mutex::new(Tree::<MERKLE_TREE_DEPTH>::new()));

//         let ban_tree: Arc<Mutex<Tree<MERKLE_TREE_DEPTH>>> =
//             Arc::new(Mutex::new(Tree::<MERKLE_TREE_DEPTH>::new()));

//         let root_hash = notes_tree.lock().root_hash().to_base();

//         let prover = Prover::new(contract, env.evm_secret_key, notes_tree, ban_tree);

//         let inputs = [
//             InputNote::<MERKLE_TREE_DEPTH>::padding_note(),
//             InputNote::padding_note(),
//         ];
//         let outputs = [Note::padding_note(), Note::padding_note()];
//         let utxo_proof = Utxo::new(inputs, outputs, root_hash, UtxoKind::Transfer);
//         let utxo_params = read_utxo_params();

//         // Prove
//         let snark = utxo_proof.snark(&utxo_params).unwrap();

//         prover
//             .add_tx(Transaction {
//                 proof: snark.to_witness(),
//             })
//             .unwrap();

//         prover.rollup().await.unwrap();
//     }

//     lazy_static::lazy_static! {
//         // Without this, we get "nonce too low" error when running tests in parallel:
//         // "Nonce too low. Expected nonce to be 1008 but got 1007."
//         static ref TEST_NONCE_BUG_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::new(());
//     }

//     #[tokio::test]
//     async fn add_prover() {
//         let _lock = TEST_NONCE_BUG_MUTEX.lock().await;

//         let env = get_env();
//         let contract = contract(env.rollup_contract_addr);

//         let tx = contract
//             .add_prover(&env.evm_secret_key, &Address::from_low_u64_be(0xfe))
//             .await
//             .unwrap();

//         while contract
//             .web3_client
//             .eth()
//             .transaction_receipt(tx)
//             .await
//             .unwrap()
//             .is_none()
//         {
//             tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//         }
//     }
// }
